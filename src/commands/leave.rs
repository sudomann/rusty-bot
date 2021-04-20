use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

use std::collections::HashSet;

use crate::{pug::GameMode, validation::game_mode::*, PugsWaitingToFill};

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
#[checks(ValidGameMode)]
#[aliases("l", "lv")]
#[min_args(1)]
async fn leave(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
            let game_modes_to_leave = args
                .iter::<String>()
                .filter(|arg| arg.is_ok())
                .map(|arg| arg.unwrap().to_lowercase())
                .collect::<HashSet<String>>();
            let mut game_modes_quitted: Vec<&GameMode> = Vec::default();
            for (game_mode, participants) in pugs_waiting_to_fill_in_guild.iter_mut() {
                if game_modes_to_leave.contains(game_mode.key()) {
                    if participants.remove(&msg.author.id) {
                        game_modes_quitted.push(game_mode);
                    }
                }
            }
            if game_modes_quitted.len() != 0 {
                msg.reply(
                    &ctx.http,
                    format!(
                        "You were removed from {:?} because you left.",
                        game_modes_quitted
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
            let mut game_modes_quitted: Vec<&GameMode> = Vec::default();
            for (game_mode, participants) in pugs_waiting_to_fill_in_guild.iter_mut() {
                if participants.remove(&msg.author.id) {
                    game_modes_quitted.push(game_mode);
                }
            }
            if game_modes_quitted.len() != 0 {
                msg.reply(
                    &ctx.http,
                    format!(
                        "You were removed from {:?} because you left.",
                        game_modes_quitted
                    ),
                )
                .await?;
            }
        }
    }
    Ok(())
}
