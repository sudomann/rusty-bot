use crate::{
    checks::{bot_voice_permission::*, voice_connected::*},
    data_structure::CompletedPug,
};
use chrono::Utc;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};
use std::collections::HashSet;

/// Maximum allowed age of a pug that this command is intended to operate upon
const NINETY_MINUTES: i64 = 5400;

// TODO: use IsConnectedToVoice check on the voice channel set/unset commands
#[command]
#[checks(IsConnectedToVoice, BotHasVoicePermission)]
/// Select users from the last pug you participated in and move them to your current voice channel
/// if they are connected to some other voice channel.
///
/// **Take note:** Only works if it has not been more than 90 minutes since the last pug that you joined filled.
async fn assemble(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let lock_for_completed_pugs = data
        .get::<CompletedPug>()
        .expect("Expected CompletedPug in TypeMap");
    let completed_pugs = lock_for_completed_pugs.read().await;
    let completed_pugs_in_guild = completed_pugs.get(&guild_id).unwrap();

    // exit early if the most recent pug is older than maximum allowed age
    if let Some(most_recent_pug) = completed_pugs_in_guild.last() {
        let time_elapsed_since_completed =
            Utc::now().timestamp() - most_recent_pug.get_created().timestamp();
        if time_elapsed_since_completed > NINETY_MINUTES {
            let response = format!(
                "There have been no completed pugs within the last {} minutes.",
                NINETY_MINUTES / 60
            );
            msg.reply(ctx, response).await?;
            return Ok(());
        }
    } else {
        msg.reply(ctx, "No pugs so far").await?;
        return Ok(());
    };

    // cycle through picking sessions (starting from most recent) and find most recent one which has the message author in it
    match completed_pugs_in_guild
        .iter()
        .rev()
        .find(|completed_pug| completed_pug.get_player_list().contains(&msg.author.id))
    {
        Some(picking_session) => {
            assemble_helper(ctx, msg, picking_session.get_player_list()).await?;
        }
        None => {
            // TODO: fetch a completed pugs from db which isnt't older than 90 mins AND this user participated in
            let found_valid_session_in_db = true;
            if found_valid_session_in_db {
                // assemble_helper(ctx, msg, player_list).await?;
            } else {
                let response = format!(
                    "You have not participated in any pugs that completed within the last {} minutes.",
                    NINETY_MINUTES / 60
                );
                msg.reply(ctx, response).await?;
            }
        }
    }
    Ok(())
}

async fn assemble_helper(
    ctx: &Context,
    msg: &Message,
    players: &HashSet<UserId>,
) -> Result<(), SerenityError> {
    let guild = msg.guild(&ctx.cache).await.unwrap();

    let voice_state = guild.voice_states.get(&msg.author.id).expect("Expected IsConnectedToVoice check to have guaranteed an entry exists for the user in guild.voice_states");
    let voice_channel_to_assemble_in = voice_state.channel_id.expect("Expected IsConnectedToVoice check to have guaranteed user is connected to some voice channel");

    let mut afk_users: Vec<UserId> = Vec::default();
    if let Some(channel_id) = guild.afk_channel_id {
        let afk_voice_channel = channel_id.to_channel(ctx).await?.guild().unwrap();
        for member in afk_voice_channel.members(ctx).await? {
            afk_users.push(member.user.id);
        }
    }

    for user_id in players {
        if afk_users.contains(user_id) {
            continue;
        }
        let _ = guild
            .id
            .move_member(ctx, user_id, voice_channel_to_assemble_in)
            .await;
    }
    Ok(())
}
