use crate::data_structure::{CompletedPug, DefaultVoiceChannels};
use chrono::Utc;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

enum Team {
    Blue,
    Red,
}

enum Action {
    Set,
    Unset,
}

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

    let data = ctx.data.read().await;

    let lock_for_default_voice = data
        .get::<DefaultVoiceChannels>()
        .expect("Expected DefaultVoiceChannels in TypeMap");
    let default_voice_channels = lock_for_default_voice.read().await;
    let default_voice_channels_in_guild = default_voice_channels.get(&guild_id).unwrap();

    let lock_for_completed_pugs = data
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

    let time_elapsed_since_filled = session.get_created() - Utc::now();
    if time_elapsed_since_filled.num_minutes() > 30 {
        msg.reply(ctx, "It's been over 30 minutes since the last pug filled")
            .await?;
        return Ok(());
    }

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
                "This server does not have a voice channel set for blue team. \
                Contact admins to run `.bluechannel`",
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
                "This server does not have a voice channel set for red team. \
                Contact admins to run `.redchannel`",
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

#[command("bluechannel")]
#[sub_commands(unset_blue_team_default_voice_channel)]
// #[allowed_roles("admin", "voice_channel_admin")]
/// Used to set the voice channel which will be used as the default voice channel for blue team.
/// Join that voice channel and then run this command.
async fn set_blue_team_default_voice_channel(
    ctx: &Context,
    msg: &Message,
    _: Args,
) -> CommandResult {
    let response = team_channel_helper(ctx, msg, Team::Blue, Action::Set).await;
    msg.reply(ctx, response).await?;
    Ok(())
}

#[command("unset")]
/// Used to unset the voice channel which will be used as the default voice channel for blue team
async fn unset_blue_team_default_voice_channel(
    ctx: &Context,
    msg: &Message,
    _: Args,
) -> CommandResult {
    let response = team_channel_helper(ctx, msg, Team::Blue, Action::Unset).await;
    msg.reply(ctx, response).await?;
    Ok(())
}

#[command("redchannel")]
#[sub_commands(unset_red_team_default_voice_channel)]
// #[allowed_roles("admin", "voice_channel_admin")]
/// Used to set the voice channel which will be used as the default voice channel for red team.
/// Join that voice channel and then run this command.
async fn set_red_team_default_voice_channel(
    ctx: &Context,
    msg: &Message,
    _: Args,
) -> CommandResult {
    let response = team_channel_helper(ctx, msg, Team::Red, Action::Set).await;
    msg.reply(ctx, response).await?;
    Ok(())
}

#[command("unset")]
/// Used to unset the voice channel which will be used as the default voice channel for red team
async fn unset_red_team_default_voice_channel(
    ctx: &Context,
    msg: &Message,
    _: Args,
) -> CommandResult {
    let response = team_channel_helper(ctx, msg, Team::Red, Action::Unset).await;
    msg.reply(ctx, response).await?;
    Ok(())
}

async fn team_channel_helper(ctx: &Context, msg: &Message, team: Team, action: Action) -> String {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    if let Action::Unset = action {
        let lock_for_default_voice = data
            .get::<DefaultVoiceChannels>()
            .expect("Expected DefaultVoiceChannels in TypeMap");
        let mut default_voice_channels = lock_for_default_voice.write().await;
        let default_voice_channels_in_guild = default_voice_channels.get_mut(&guild_id).unwrap();
        match team {
            Team::Blue => match default_voice_channels_in_guild.unset_blue() {
                Some(c) => {
                    return format!(
                        "{} has been unset as default voice channel for the blue team",
                        c.mention()
                    );
                }
                None => {
                    return "There is no voice channel assigned as default for the blue team"
                        .to_string();
                }
            },
            Team::Red => match default_voice_channels_in_guild.unset_red() {
                Some(c) => {
                    return format!(
                        "{} has been unset as default voice channel for the red team",
                        c.mention()
                    );
                }
                None => {
                    return "There is no voice channel assigned as default for the red team"
                        .to_string();
                }
            },
        }
    }

    match msg.guild(&ctx).await {
        Some(guild) => {
            match guild.voice_states.get(&msg.author.id) {
                Some(voice_state) => {
                    if voice_state.channel_id.is_none() {
                        return "You need to be connected to the voice channel you want to set"
                            .to_string();
                    }
                    let voice_channel_to_set = voice_state.channel_id.unwrap();
                    let lock_for_default_voice = data
                        .get::<DefaultVoiceChannels>()
                        .expect("Expected DefaultVoiceChannels in TypeMap");
                    let mut default_voice_channels = lock_for_default_voice.write().await;
                    let default_voice_channels_in_guild =
                        default_voice_channels.get_mut(&guild_id).unwrap();

                    match team {
                        Team::Blue => {
                            let red_channel = default_voice_channels_in_guild.get_red();

                            // Validate that the voice channel the user is trying to set has not
                            // already been assigned to red team
                            if red_channel.is_some() && red_channel.unwrap() == voice_channel_to_set
                            {
                                return "The channel you're in has already been assigned to red team"
                            .to_string();
                            }

                            default_voice_channels_in_guild.set_blue(voice_channel_to_set);
                            return format!(
                                "Default voice channel for blue team has been set to {}",
                                voice_channel_to_set.mention()
                            );
                        }
                        Team::Red => {
                            let blue_channel = default_voice_channels_in_guild.get_blue();

                            // Validate that the voice channel the user is trying to set has not
                            // already been assigned to red team
                            if blue_channel.is_some()
                                && blue_channel.unwrap() == voice_channel_to_set
                            {
                                return "The channel you're in has already been assigned to blue team"
                            .to_string();
                            }
                            default_voice_channels_in_guild.set_red(voice_channel_to_set);
                            return format!(
                                "Default voice channel for red team has been set to {}",
                                voice_channel_to_set.mention()
                            );
                        }
                    }
                }
                None => {
                    return "You need to be connected to the voice channel you want to set"
                        .to_string();
                }
            }
        }
        None => {
            return "Sorry, that didn't work because some data is missing".to_string();
        }
    }
}
