use std::collections::HashSet;
use std::sync::Arc;

use chrono::{Duration, Utc};
use serenity::all::{CreateEmbed, CreateMessage};
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use tracing::{error, info, instrument};

use crate::db::write::mark_voice_channels_deleted;

#[instrument(skip(ctx))]
pub async fn log_system_load(ctx: Arc<Context>) {
    let cpu_load = sys_info::loadavg().unwrap();
    let mem_use = sys_info::mem_info().unwrap();
    let m = CreateMessage::new().add_embed(
        CreateEmbed::new()
            .title("System Resource Load")
            .field(
                "CPU Load Average",
                format!("{:.2}%", cpu_load.one * 10.0),
                false,
            )
            .field(
                "Memory Usage",
                format!(
                    "{:.2} MB Free out of {:.2} MB",
                    mem_use.free as f32 / 1000.0,
                    mem_use.total as f32 / 1000.0
                ),
                false,
            ),
    );

    if let Err(why) = UserId::from(209721904662183937)
        .create_dm_channel(&*ctx)
        .await
        .expect("expected opened dm channel with sudomann")
        .send_message(&ctx, m)
        .await
    {
        eprintln!("Error sending message: {:?}", why);
    };
}

/// Remove players from pug if they joined over 6 hours ago.
///
/// Since currently there is no enforcing of processing pug commands
/// in a designated pug channel, when a designated pug channel is not declared,
/// a DM will be sent to the removed user instead of in a pug channel
#[instrument(skip(ctx))]
pub async fn clear_out_stale_joins(ctx: Arc<Context>) {
    let current_time = Utc::now();
    let _formatted_time = current_time.to_rfc2822();

    let db_client = {
        let data = ctx.data.read().await;
        match data.get::<crate::DbClientRef>() {
            Some(c) => c.clone(),
            None => {
                info!(
                    "Client for database was not available - skipping stale voice channel removal"
                );
                return;
            }
        }
    };

    let guilds = ctx.cache.guilds();
    let mut has_error = false;
    let mut job_log = MessageBuilder::default();
    for guild_id in guilds {
        let client = db_client.clone();
        let guild_db = client.database(guild_id.get().to_string().as_str());
        let mut temp_log = MessageBuilder::default();
        match guild_id.name(&ctx) {
            Some(name) => {
                temp_log.push(name);
            }
            None => {
                temp_log.push(guild_id.to_string());
            }
        }
        temp_log.push_line(":").push_line("==============");

        match crate::db::read::get_stale_game_mode_joins(guild_db.clone(), Duration::hours(4)).await
        {
            Ok(game_mode_joins) => {
                if game_mode_joins.is_empty() {
                    continue;
                }
                match crate::db::read::get_pug_channel(guild_db.clone()).await {
                    Ok(maybe_pug_channel) => {
                        let mut removed_users: HashSet<UserId> = HashSet::default();
                        for join in game_mode_joins {
                            let u_id = join.player_user_id as u64;
                            removed_users.insert(UserId::from(u_id));
                            tokio::spawn(crate::db::write::remove_player_from_game_mode_queue(
                                guild_db.clone(),
                                join.game_mode_label,
                                u_id,
                            ));
                        }

                        // delete joins
                        match maybe_pug_channel {
                            Some(pug_channel) => {
                                // send msg in channel
                                let mut msg = MessageBuilder::default();
                                msg.push_line("Players removed due to timeout:");
                                for user in removed_users {
                                    msg.mention(&user).push(" ");
                                }

                                let _ = ChannelId::from(pug_channel.channel_id as u64)
                                    .say(&ctx.http, msg.build())
                                    .await;
                            }
                            None => {
                                // send dm to removed users
                                for user in removed_users {
                                    match user.create_dm_channel(&ctx.http).await {
                                        Ok(c) => {
                                            let _ = c
                                                .say(
                                                    &ctx.http,
                                                    "You have been removed from pug due to timeout",
                                                )
                                                .await;
                                        }
                                        Err(err) => {
                                            has_error = true;
                                            temp_log.push_line("Expected joined player to have a valid userid for dm:")
                                                    .push_line(err.to_string());
                                        }
                                    };
                                }
                            }
                        }
                    }
                    Err(err) => {
                        has_error = true;
                        temp_log
                            .push_line("Failed to read pug channel")
                            .push_line(err.to_string());
                    }
                }
            }
            Err(err) => {
                has_error = true;
                temp_log
                    .push_line("Failed to read stale game mode joins")
                    .push_line(err.to_string());
            }
        }
        if has_error {
            job_log.push(temp_log.build());
        }
    }
    let job_log_output = job_log.build();
    if has_error {
        error!("{}", job_log_output);
    } else {
        info!("{}", job_log_output);
    }
}

#[instrument(skip(ctx))]
pub async fn remove_stale_team_voice_channels(ctx: Arc<Context>) {
    // !TODO: make sure to skip deleting a voice channel if it's not empty??
    let db_client = {
        let data = ctx.data.read().await;
        match data.get::<crate::DbClientRef>() {
            Some(c) => c.clone(),
            None => {
                info!(
                    "Client for database was not available - skipping stale voice channel removal"
                );
                return;
            }
        }
    };

    let guilds = ctx.cache.guilds();

    for guild_id in guilds {
        let ctx_clone = ctx.clone();

        let guild_db = db_client.database(guild_id.get().to_string().as_str());
        let mut has_error = false;
        let mut job_log = MessageBuilder::default();
        match guild_id.name(&ctx.cache) {
            Some(guild_name) => job_log.push_line(guild_name),
            None => job_log.push_line(guild_id.to_string()),
        };

        match crate::db::read::get_voice_channels_pending_deletion(
            guild_db.clone(),
            //chrono::Duration::hours(2),
            Duration::seconds(5),
        )
        .await
        {
            Ok(team_voice_channels) => {
                if team_voice_channels.is_empty() {
                    job_log.push_line("No stale voice channels found");
                    break;
                }
                let mut deleted: Vec<i64> = Vec::default();
                for channel_set in team_voice_channels {
                    // gather list of all those channel ids that were either deleted/unkown
                    // FIXME: make db request to flip booleans to true (mark as deleted)
                    // for documents where at least 1/3 of its voice channel ids are in the list

                    for id in vec![
                        channel_set.category.id as u64,
                        channel_set.blue_channel.id as u64,
                        channel_set.red_channel.id as u64,
                    ] {
                        match ctx_clone
                            .http
                            .delete_channel(ChannelId::from(id), None)
                            .await
                        {
                            Ok(channel) => {
                                let (kind , name) = match channel {
                                    serenity::model::channel::Channel::Guild(guild_channel) => {
                                        match guild_channel.kind {
                                            serenity::model::channel::ChannelType::Category => ("category", guild_channel.name),
                                            serenity::model::channel::ChannelType::Voice => ("channel", guild_channel.name),
                                            _ => panic!("Somehow a guild channel which is not of either Category or Voice kind was being evaluted for \"if stale then delete\"")
                                        }
                                    },
                                    _ => panic!("Somehow a `serenity::model::channel::Channel` which is not of either `Category` or `Guild` variant was being evaluted for \"if stale then delete\"")
                                };

                                job_log
                                    .push_line(format!("Successfully deleted {} - {}", kind, name));
                                deleted.push(id as i64);
                            }
                            Err(err) => {
                                has_error = true;
                                job_log
                                    .push(format!("Failed to delete channel/category {}: ", id))
                                    .push_line(err.to_string());
                            }
                        }
                    }
                }
                if !deleted.is_empty() {
                    mark_voice_channels_deleted(guild_db, deleted).await;
                }
            }
            Err(err) => {
                has_error = true;
                job_log.push_line("Failed to read voice channels pending deletion:");
                job_log.push(err.to_string());
            }
        }
        let job_log_output = job_log.build();
        if has_error {
            error!("{}", job_log_output);
        } else {
            info!("{}", job_log_output);
        }
    }
}

#[instrument(skip(ctx))]
pub async fn remove_stale_threads(ctx: &Arc<Context>, guilds: &Vec<GuildId>) {
    // !TODO: skip deleting a voice channel if it's not empty
    let db_client = {
        let data = ctx.data.read().await;
        match data.get::<crate::DbClientRef>() {
            Some(c) => c.clone(),
            None => {
                info!("Client for database was not available - skipping stale thread removal");
                return;
            }
        }
    };

    for guild_id in guilds {
        // TODO: this job does nothing yet
    }
}
