use std::collections::HashSet;

use crate::{CompletedPug, DefaultVoiceChannels};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[aliases("voice", "eugene")]
#[max_args(0)]
#[allowed_roles("admin", "voice_channel_admin")]
/// Moves pug participants into their team voice channels if they are not in them already.
/// Only works on people that are already connected to voice chat
async fn voices(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let bot_id = match ctx.http.get_current_application_info().await {
        Ok(info) => info.id,
        Err(why) => panic!("Could not access application info: {:?}", why),
    };
    let bot_membership = guild_id.member(ctx, bot_id).await?;
    let has_move_members_perm = bot_membership
        .permissions(ctx)
        .await
        .expect("Expected to get bot permissions in guild")
        .move_members();

    if !has_move_members_perm {
        msg.channel_id
            .say(
                ctx,
                "I don't have the `Move Members` permission :( \
            Please contact an admin to fix this so I can move players",
            )
            .await?;
        return Ok(());
    }

    let data_write = ctx.data.read().await;

    let lock_for_default_voice = data_write
        .get::<DefaultVoiceChannels>()
        .expect("Expected DefaultVoiceChannels in TypeMap");
    let default_voice_channels = lock_for_default_voice.read().await;
    let default_voice_channels_in_guild = default_voice_channels.get(&guild_id).unwrap();

    let lock_for_completed_pugs = data_write
        .get::<CompletedPug>()
        .expect("Expected FilledPug in TypeMap");

    let completed_pugs = lock_for_completed_pugs.read().await;

    let completed_pugs_in_guild = completed_pugs.get(&guild_id).unwrap();

    let last_pug = completed_pugs_in_guild.last();
    if last_pug.is_none() {
        msg.reply(ctx, "No completed pugs").await?;
        return Ok(());
    }
    let session = last_pug.unwrap();

    // TODO: this command shouldnt be useable beyond 5 mins post pug completion

    let afk_channel = match guild_id.to_guild_cached(ctx).await {
        Some(guild) => guild.afk_channel_id,
        None => {
            let guild = guild_id.to_partial_guild(ctx).await?;
            guild.afk_channel_id
        }
    };
    let mut excluded_players: Vec<UserId> = Vec::default();
    if let Some(channel_id) = afk_channel {
        let afk_voice_channel = channel_id.to_channel(ctx).await?.guild().unwrap();
        for member in afk_voice_channel.members(ctx).await? {
            excluded_players.push(member.user.id);
        }
    }

    let blue_voice = default_voice_channels_in_guild.get_blue();
    if blue_voice.is_none() {
        let _ = msg
            .reply(
                ctx,
                "This channel does not have a voice channel set for blue team",
            )
            .await;
    } else {
        for (_, player) in session.get_blue_team() {
            if excluded_players.contains(player) {
                continue;
            }
            let _ = guild_id.move_member(ctx, player, blue_voice.unwrap()).await;
        }
    }

    let red_voice = default_voice_channels_in_guild.get_red();
    if red_voice.is_none() {
        let _ = msg
            .reply(
                ctx,
                "This channel does not have a voice channel set for red team",
            )
            .await;
    } else {
        for (_, player) in session.get_red_team() {
            if excluded_players.contains(player) {
                continue;
            }
            let _ = guild_id.move_member(ctx, player, red_voice.unwrap()).await;
        }
    }

    Ok(())
}
