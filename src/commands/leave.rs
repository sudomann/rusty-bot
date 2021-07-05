use crate::{
    checks::{pug_channel::*, sync_in_progress::*},
    data_structure::{FilledPug, PugsWaitingToFill, RegisteredGameModes},
    pug::{game_mode::GameMode, player::Player},
    utils::{
        parse_game_modes::{parse_game_modes, GameModeError, ParsedGameModes},
        player_user_ids_to_users::player_user_ids_to_users,
    },
};
use itertools::Itertools;
use linked_hash_set::LinkedHashSet;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

/// Perform leave operations and compose a String detailing results of actions taken
async fn leave_handler(
    ctx: &Context,
    game_modes_to_leave: ParsedGameModes,
    user_to_remove: User,
    guild_id: GuildId,
) -> Result<String, SerenityError> {
    let mut unfilled_pugs_removed_from: Vec<GameMode> = Vec::default();
    let mut filled_pugs_removed_from: Vec<GameMode> = Vec::default();
    let mut vacated_players: Vec<(GameMode, LinkedHashSet<Player>)> = Vec::default();
    {
        let data = ctx.data.read().await;

        // leave unfilled pugs first
        let lock_for_pugs_waiting_to_fill = data
            .get::<PugsWaitingToFill>()
            .expect("Expected PugsWaitingToFill in TypeMap");

        let mut pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.write().await;
        let pugs_waiting_to_fill_in_guild = pugs_waiting_to_fill.get_mut(&guild_id).unwrap();
        for game_mode in &game_modes_to_leave {
            let participants = pugs_waiting_to_fill_in_guild.get_mut(game_mode).unwrap();
            if participants.remove(&user_to_remove.id) {
                unfilled_pugs_removed_from.push(game_mode.clone());
            }
        }

        // then leave filled pugs
        let lock_for_filled_pugs = data
            .get::<FilledPug>()
            .expect("Expected CompletedPug in TypeMap");
        let mut filled_pugs = lock_for_filled_pugs.write().await;
        let filled_pugs_in_guild = filled_pugs.get_mut(&guild_id).unwrap();

        for i in (0..filled_pugs_in_guild.len()).rev() {
            let filled_pug = filled_pugs_in_guild.get(i).unwrap();
            if game_modes_to_leave.contains(filled_pug.get_game_mode()) {
                // it was filled, but now it's cancelled
                let mut cancelled_pug = filled_pugs_in_guild.remove(i).unwrap();
                let cancelled_game_mode = cancelled_pug.get_game_mode().clone();
                filled_pugs_removed_from.push(cancelled_game_mode.clone());
                // In case picking had already begun, reset
                // This moves all players into one collection of unpicked players
                // so we can get them with .get_remaining()
                cancelled_pug.reset();
                let users_in_cancelled_pug =
                    player_user_ids_to_users(ctx, cancelled_pug.get_remaining()).await?;
                let unfilled_pug = pugs_waiting_to_fill_in_guild
                    .remove(&cancelled_game_mode)
                    .unwrap();

                if !unfilled_pug.is_empty() {
                    // vacate players that had joined this game mode
                    // after the current, cancelled pug filled
                    vacated_players.push((cancelled_game_mode.clone(), unfilled_pug));
                }

                let mut cancelled_pug_players = LinkedHashSet::default();

                // The players in the cancelled pug will be reinserted to unfilled pug list
                let move_to_unfilled_pug = |(_, user)| {
                    // exclude the player leaving
                    if user != user_to_remove {
                        cancelled_pug_players.insert(Player::new(user));
                    }
                };
                users_in_cancelled_pug
                    .into_iter()
                    .for_each(move_to_unfilled_pug);
                pugs_waiting_to_fill_in_guild
                    .insert(cancelled_game_mode.clone(), cancelled_pug_players);
            }
        }
    }
    let mut message = MessageBuilder::new();

    if unfilled_pugs_removed_from.is_empty() && filled_pugs_removed_from.is_empty() {
        message.push(user_to_remove);
        message.push(" wasn't found in any pug");
        return Ok(message.build());
    }

    if !unfilled_pugs_removed_from.is_empty() {
        let a = unfilled_pugs_removed_from
            .iter()
            .format_with(" :small_orange_diamond: ", |g, f| {
                f(&format_args!("{}", g.label()))
            });
        message.push(user_to_remove);
        message.push(" has been removed from ").push_line(a);
    }

    if !filled_pugs_removed_from.is_empty() {
        let a = filled_pugs_removed_from
            .iter()
            .format_with(" :small_orange_diamond: ", |g, f| {
                f(&format_args!("{}", g.label()))
            });
        message
            .push_line("")
            .push(a)
            .push_bold_line(" cancelled")
            .push("TODO: ping vacated players");
    }

    Ok(message.build())
}

#[command("l")]
#[checks(PugChannel, GuildDataSyncInProgress)]
#[aliases("lv", "leave")]
#[min_args(1)]
pub async fn leave(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let game_modes_to_leave = match parse_game_modes(ctx, &guild_id, args.clone()).await {
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
    let response = leave_handler(ctx, game_modes_to_leave, msg.author.clone(), guild_id).await?;
    let _ = msg.reply(&ctx.http, response).await;
    Ok(())
}

#[command("lva")]
#[checks(PugChannel, GuildDataSyncInProgress)]
async fn leave_all(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let game_modes_to_leave = {
        let data_read = ctx.data.read().await;
        let lock_for_registered_game_modes = data_read
            .get::<RegisteredGameModes>()
            .expect("Expected RegisteredGameModes in TypeMap")
            .clone();
        let global_game_modes = lock_for_registered_game_modes.read().await;

        let guild_game_modes = global_game_modes.get(&guild_id);
        if guild_game_modes.is_none() {
            msg.reply(
                ctx,
                "No game modes registered. Contact admins to run `.addmod`",
            )
            .await?;
            return Ok(());
        }
        guild_game_modes.unwrap().clone()
    };
    let response = leave_handler(ctx, game_modes_to_leave, msg.author.clone(), guild_id).await?;
    let _ = msg.reply(&ctx.http, response).await;
    Ok(())
}
