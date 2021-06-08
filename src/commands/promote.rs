use crate::{
    data_structure::PugsWaitingToFill,
    utils::parse_game_modes::{parse_game_modes, GameModeError},
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

#[command]
#[aliases("pro")]
#[min_args(1)]
#[max_args(1)]
// TODO: admin configurable rate limiting (by number of invocations per minute, no matter who)
/// Pinging `@here` with a message promoting a single game mode
// "@here" is backticked so when this help text is printed in an embed it doesn't actually try to ping
async fn promote(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    match parse_game_modes(ctx, &guild_id, args).await {
        Ok(game_modes) => {
            let lock_for_pugs_waiting_to_fill = {
                let data_read = ctx.data.read().await;
                data_read
                    .get::<PugsWaitingToFill>()
                    .expect("Expected PugsWaitingToFill in TypeMap")
                    .clone()
            };
            let pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.read().await;
            let pugs = pugs_waiting_to_fill.get(&guild_id).unwrap();

            for game_mode in game_modes {
                let maybe_participants = pugs.get(&game_mode);
                if maybe_participants.is_none() {
                    msg.reply(
                        &ctx.http,
                        "Valid game mode, but for some reason, \
                it's absent from map of pugs one can join. This should not happen. \
                Contact sudomann#9568 to investigate.",
                    )
                    .await?;
                    return Ok(());
                }
                let p = maybe_participants.unwrap();
                let spots_remaining = game_mode.capacity() - (p.len() as u8);
                msg.channel_id
                    .say(
                        &ctx.http,
                        MessageBuilder::new()
                            .push("@here ")
                            .push(spots_remaining)
                            .push(" more needed for ")
                            .push(game_mode.to_string())
                            .build(),
                    )
                    .await?;
            }
        }
        Err(err) => {
            match err {
                GameModeError::NoneGiven(m)
                | GameModeError::NoneRegistered(m)
                | GameModeError::Foreign(m) => msg.reply(&ctx.http, m).await?,
            };
        }
    }

    Ok(())
}
