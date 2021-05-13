use crate::ShardManagerContainer;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[owners_only]
async fn quit(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        msg.reply(ctx, "Shutting down! - No memory will be persisted")
            .await?;
        manager.lock().await.shutdown_all().await;
    } else {
        msg.reply(ctx, "There was a problem getting the shard manager")
            .await?;

        return Ok(());
    }

    Ok(())
}

#[command]
#[owners_only]
/// Persist all data and then shut down
///
/// Locks the global data map so no more command can be executed while
/// data is saved to persistent storage.
/// Announces in all guilds that the bot is going down for maintenance.
async fn maintenance(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Shutting down gracefully...").await?;
    let _data = ctx.data.write().await;

    Ok(())
}
