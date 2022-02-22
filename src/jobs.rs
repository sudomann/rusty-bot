use std::sync::Arc;

use chrono::Utc;
use serenity::model::id::{GuildId, UserId};
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use tracing::{error, info, instrument};

#[instrument(skip(ctx))]
pub async fn log_system_load(ctx: Arc<Context>) {
    let cpu_load = sys_info::loadavg().unwrap();
    let mem_use = sys_info::mem_info().unwrap();

    if let Err(why) = UserId(209721904662183937)
        .create_dm_channel(&*ctx)
        .await
        .expect("expected opened dm channel with sudomann")
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title("System Resource Load");
                e.field(
                    "CPU Load Average",
                    format!("{:.2}%", cpu_load.one * 10.0),
                    false,
                );
                e.field(
                    "Memory Usage",
                    format!(
                        "{:.2} MB Free out of {:.2} MB",
                        mem_use.free as f32 / 1000.0,
                        mem_use.total as f32 / 1000.0
                    ),
                    false,
                );
                e
            })
        })
        .await
    {
        eprintln!("Error sending message: {:?}", why);
    };
}

/// Remove players from pug if they joined over 6 hours ago
#[instrument(skip(_ctx))]
pub async fn clear_out_stale_joins(_ctx: Arc<Context>) {
    let current_time = Utc::now();
    let _formatted_time = current_time.to_rfc2822();

    // _ctx.set_activity(Activity::playing(&_formatted_time)).await;
}

#[instrument(skip(ctx))]
pub async fn remove_stale_team_voice_channels(ctx: &Arc<Context>, guilds: &Vec<GuildId>) {
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

    for guild_id in guilds {
        let ctx_clone = ctx.clone();

        let guild_db = db_client.database(guild_id.0.to_string().as_str());
        let mut has_error = false;
        let mut job_log = MessageBuilder::default();
        job_log.push_line(guild_id);
        match crate::db::read::get_voice_channels_pending_deletion(
            guild_db,
            chrono::Duration::hours(2),
        )
        .await
        {
            Ok(team_voice_channels) => {
                for channel_set in team_voice_channels {
                    match ctx_clone
                        .http
                        .delete_channel(channel_set.category_id.parse::<u64>().unwrap())
                        .await
                    {
                        Ok(channel) => {
                            let category = channel.category().unwrap();
                            job_log.push_line(format!(
                                "Successfully deleted the {} category",
                                category.name
                            ));
                        }
                        Err(err) => {
                            job_log
                                .push_line("Failed to delete an item:")
                                .push_line(err);
                        }
                    }

                    match ctx_clone
                        .http
                        .delete_channel(channel_set.blue_channel_id.parse::<u64>().unwrap())
                        .await
                    {
                        Ok(channel) => {
                            let blue_team_voice_channel = channel.guild().unwrap();
                            job_log.push_line(format!(
                                "Successfully deleted the {} channel",
                                blue_team_voice_channel.name
                            ));
                        }
                        Err(err) => {
                            job_log
                                .push_line("Failed to delete an item:")
                                .push_line(err);
                        }
                    }

                    match ctx_clone
                        .http
                        .delete_channel(channel_set.red_channel_id.parse::<u64>().unwrap())
                        .await
                    {
                        Ok(channel) => {
                            let red_team_voice_channel = channel.guild().unwrap();
                            job_log.push_line(format!(
                                "Successfully deleted the {} channel",
                                red_team_voice_channel.name
                            ));
                        }
                        Err(err) => {
                            job_log
                                .push_line("Failed to delete an item:")
                                .push_line(err);
                        }
                    }
                }
            }
            Err(err) => {
                has_error = true;
                job_log.push_line("Failed to read voice channels pending deletion:");
                job_log.push(err);
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
