use chrono::Utc;
use serenity::{
    model::{gateway::Activity, id::UserId},
    prelude::*,
};
use std::sync::Arc;

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
pub async fn clear_out_stale_joins(_ctx: Arc<Context>) {
    let current_time = Utc::now();
    let _formatted_time = current_time.to_rfc2822();

    // _ctx.set_activity(Activity::playing(&_formatted_time)).await;
}
