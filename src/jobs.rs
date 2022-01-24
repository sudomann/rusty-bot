use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serenity::model::id::{GuildId, UserId};
use serenity::prelude::*;
use tokio::time::interval;
use tracing::{info, instrument};

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
    // !TODO: skip deleting a voice channel if it's not empty
    let db_client = {
        let data = ctx.data.read().await;
        match data.get::<crate::DbClientRef>() {
            Some(c) => c.clone(),
            None => {
                info!("Client for database was not available - skipping this round");
                return;
            }
        }
    };

    for guild_id in guilds {
        tokio::spawn(async {
            // let guild_db = db_client.clone().database(guild_id.0.as_str());
            // TODO: voice channels must be manually cleaned up from testing till you complete this job
            // crate::db::read::
        });
    }
}
