use crate::{
    pug::{game_mode::GameMode, player::Players},
    utils::parse_game_modes::{parse_game_modes, GameModeError},
    PugsWaitingToFill, RegisteredGameModes,
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

const MIN_PUG_PARTICIPANT_COUNT: u8 = 2;
const UT_MAX_PLAYER_COUNT: u8 = 24;

#[command("addmod")]
#[max_args(2)]
#[min_args(2)]
/// Register a game mode with player count
///
/// `.addmod <label> <player count>`
///
/// Example `.addmod ctf 10`
///
/// The label cannot be composed entirely of numbers
///
/// The player count must be an even number in the range of 2 - 24
async fn register_game_mode(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let guild_id = msg.guild_id.unwrap();
    let lock_for_registered_game_modes = data
        .get::<RegisteredGameModes>()
        .expect("Expected RegisteredGameModes in TypeMap");
    let mut registered_game_modes = lock_for_registered_game_modes.write().await;
    let registered_game_modes_in_guild = registered_game_modes.get_mut(&guild_id).unwrap();
    let label = args.single_quoted::<String>().unwrap();

    if label.bytes().all(|x| x.is_ascii_digit()) {
        msg.reply(ctx, "The game mode label cannot be just a number")
            .await?;
        return Ok(());
    }

    if registered_game_modes_in_guild.contains(&label) {
        msg.reply(ctx, "The label you privded has already been registered")
            .await?;
        return Ok(());
    }

    match args.single_quoted::<u8>() {
        Ok(player_count) => {
            if player_count % 2 != 0 {
                msg.reply(ctx, "Player count must be an even number")
                    .await?;
                return Ok(());
            }
            if player_count < MIN_PUG_PARTICIPANT_COUNT {
                msg.reply(
                    ctx,
                    format!(
                        "Player count cannot be less than {}",
                        MIN_PUG_PARTICIPANT_COUNT
                    ),
                )
                .await?;
                return Ok(());
            }
            if player_count > UT_MAX_PLAYER_COUNT {
                msg.reply(
                    ctx,
                    format!(
                        "Player count cannot be greater than {}",
                        UT_MAX_PLAYER_COUNT
                    ),
                )
                .await?;
                return Ok(());
            }

            let new_game_mode = GameMode::new(label, player_count);
            // add new game mode to both RegisteredGameModes and PugsWaitingToFill
            let lock_for_pugs_waiting_to_fill = data
                .get::<PugsWaitingToFill>()
                .expect("Expected PugsWaitingToFill in TypeMap");
            let mut pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.write().await;
            let potential_pugs = pugs_waiting_to_fill.get_mut(&guild_id).unwrap();
            potential_pugs.insert(new_game_mode.clone(), Players::default());
            registered_game_modes_in_guild.insert(new_game_mode);
            msg.reply(ctx, "Game mode registered").await?;
        }
        Err(err) => {
            msg.reply(ctx, err).await?;
        }
    }

    Ok(())
}

#[command("delmod")]
#[max_args(1)]
#[min_args(1)]
// TODO: maybe add support for deleting multiple game modes at once?
async fn delete_game_mode(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    match parse_game_modes(ctx, &guild_id, args).await {
        Ok(game_modes_to_delete) => {
            let data = ctx.data.read().await;
            let lock_for_registered_game_modes = data
                .get::<RegisteredGameModes>()
                .expect("Expected RegisteredGameModes in TypeMap");
            let mut registered_game_modes = lock_for_registered_game_modes.write().await;
            let registered_game_modes_in_guild = registered_game_modes
                .get_mut(&msg.guild_id.unwrap())
                .unwrap();

            let lock_for_pugs_waiting_to_fill = data
                .get::<PugsWaitingToFill>()
                .expect("Expected PugsWaitingToFill in TypeMap");
            let mut pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.write().await;
            let potential_pugs = pugs_waiting_to_fill.get_mut(&guild_id).unwrap();

            // this loop will only run once, because this command only allows a single argument,
            // and thus the parsing function can return a hashset with no more than one element
            for game_mode in game_modes_to_delete.iter() {
                if registered_game_modes_in_guild.remove(game_mode)
                    && potential_pugs.remove(game_mode).is_some()
                {
                    msg.reply(ctx, "Game mode removed").await?;
                } else {
                    msg.reply(ctx, "Deletion did not complete successfully")
                        .await?;
                }
            }
        }
        Err(err) => match err {
            GameModeError::NoneGiven(m)
            | GameModeError::NoneRegistered(m)
            | GameModeError::Foreign(m) => {
                msg.reply(ctx, m).await?;
            }
        },
    }

    Ok(())
}
