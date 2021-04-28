use crate::{
    pug::game_mode::GameMode,
    utils::parse_game_modes::{parse_game_modes, GameModeError},
    PugsWaitingToFill,
};
use itertools::join;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

/*
TODO: examine whether this "behavior" technique can be used
  as an alternative to current code duplication in leave commands
trait QuitBehavior {
    fn do_behavior(a: A, b: &B);
}

struct QuitSpecificGameModes;
impl QuitBehavior for QuitSpecificGameModes {
    fn do_behavior(a: A, b: &B) {}
}
struct QuitAllGameModes;
impl QuitBehavior for QuitAllGameModes {
    fn do_behavior(a: A, b: &B) {}
}

fn leave_pugs<T: QuitBehavior>(x: X) {
    let a = qux(x);
    let b = B::new();
    T::do_behavior(a, &b);
    bar();
}

async fn quit_command() {
    leave_pugs::<QuitSpecificGameModes>(x1);
    leave_pugs::<QuitAllGameModes>(x2);
}
*/

/*  Leave command MUST NOT touch pug that has filled already
    Incapable or afk pug participants must be subbed with .substitute
    This is to keep commands simple and memorable and avoid convoluted code
*/

#[command]
#[aliases("l", "lv")]
#[min_args(1)]
pub async fn leave(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if let Some(guild_id) = msg.guild_id {
        let lock_for_pugs_waiting_to_fill = {
            let data_write = ctx.data.read().await;
            data_write
                .get::<PugsWaitingToFill>()
                .expect("Expected PugsWaitingToFill in TypeMap")
                .clone()
        };
        let mut pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.write().await;

        if let Some(pugs_waiting_to_fill_in_guild) = pugs_waiting_to_fill.get_mut(&guild_id) {
            let game_modes_to_leave =
                match parse_game_modes(ctx.clone(), guild_id, args.clone()).await {
                    Ok(game_modes) => game_modes,
                    Err(err) => {
                        match err {
                            GameModeError::NoneGiven(m)
                            | GameModeError::NoneRegistered(m)
                            | GameModeError::Foreign(m) => {
                                msg.reply(ctx, m).await?;
                            }
                        }
                        return Ok(());
                    }
                };
            let mut game_modes_actually_removed_from: Vec<&GameMode> = Vec::default();
            for (game_mode, participants) in pugs_waiting_to_fill_in_guild.iter_mut() {
                if game_modes_to_leave.contains(game_mode) {
                    if participants.remove(&msg.author.id) {
                        game_modes_actually_removed_from.push(game_mode);
                    }
                }
            }
            if !game_modes_actually_removed_from.is_empty() {
                let labels = game_modes_actually_removed_from
                    .iter()
                    .map(|g| g.label())
                    .collect::<Vec<&String>>();
                let pretty_labels = join(labels, " :small_orange_diamond: ");

                msg.reply(
                    &ctx.http,
                    format!(
                        "**{}** has been removed from {}.",
                        msg.author.name, pretty_labels
                    ),
                )
                .await?;
            }
        }
    }
    Ok(())
}

#[command]
#[aliases("lva", "leaveall")]
async fn leave_all(ctx: &Context, msg: &Message) -> CommandResult {
    if let Some(guild_id) = msg.guild_id {
        let lock_for_pugs_waiting_to_fill = {
            let data_write = ctx.data.read().await;
            data_write
                .get::<PugsWaitingToFill>()
                .expect("Expected PugsWaitingToFill in TypeMap")
                .clone()
        };
        let mut pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.write().await;

        if let Some(pugs_waiting_to_fill_in_guild) = pugs_waiting_to_fill.get_mut(&guild_id) {
            let mut game_modes_actually_removed_from: Vec<&GameMode> = Vec::default();
            for (game_mode, participants) in pugs_waiting_to_fill_in_guild.iter_mut() {
                if participants.remove(&msg.author.id) {
                    game_modes_actually_removed_from.push(game_mode);
                }
            }
            if !game_modes_actually_removed_from.is_empty() {
                let labels = game_modes_actually_removed_from
                    .iter()
                    .map(|g| g.label())
                    .collect::<Vec<&String>>();
                let pretty_labels = join(labels, " :small_orange_diamond: ");

                msg.reply(
                    &ctx.http,
                    format!("You were removed from {} because you left.", pretty_labels),
                )
                .await?;
            }
        }
    }
    Ok(())
}
