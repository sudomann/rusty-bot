use futures::future::join_all;
use futures::stream::TryStreamExt;
use mongodb::Client;
use std::sync::Arc;

use serenity::client::Context;
use serenity::model::id::GuildId;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tracing::{error, info, instrument};

use crate::db::model::GuildCommand;
use crate::DbClientRef;

/// For each guild, check for presence of guild application commands created by this bot.
/// If there aren't suitable existing commands, create a `/setup` command
#[instrument(skip(ctx, guild_ids))]
pub async fn inspect_guild_commands(ctx: Arc<Context>, guild_ids: Vec<GuildId>) {
    let mut interval = interval(Duration::from_secs(1));

    // loop/block until the Client is available in storage
    let db_client = loop {
        let data = ctx.data.read().await;
        match data.get::<DbClientRef>() {
            Some(c) => break c.clone(),
            None => {
                info!("Waiting for database client ready");
                interval.tick().await;
            }
        }
    };

    let mut join_handles: Vec<JoinHandle<Result<GuildId, crate::error::Error>>> = Vec::default();

    info!("Launching one task per connected guild for conducting inspection");
    for guild_id in guild_ids {
        join_handles.push(tokio::spawn(inspect_and_maybe_update_db(
            ctx.clone(),
            guild_id,
            db_client.clone(),
        )));
    }

    for handle in join_all(join_handles).await {
        match handle {
            Ok(result) => {
                match result {
                    Ok(_g) => {
                        // Considering that printing to stdout on every iteration is slow
                        // Is it worth announcing anything for this?
                    }
                    Err(err) => {
                        error!(
                            "Error during an attempt to write to the database:\n {:?}",
                            err
                        );
                    }
                }
            }
            Err(join_err) => {
                error!("Failed to rejoin a task:\n {:?}", join_err);
            }
        }
    }
    info!("Inspections complete!");
}

/// Register /setup command as necessary for guilds
///
/// TODO: ensure that guilds marked as disabled don't have/get any guild commands registered.
async fn inspect_and_maybe_update_db(
    ctx: Arc<Context>,
    guild_id: GuildId,
    db_client: Client,
) -> Result<GuildId, crate::error::Error> {
    let commands = guild_id.get_application_commands(&ctx.http).await?;

    let db = db_client.database("guild_commands");

    // get commands saved in db
    let mut cursor = db
        .collection(guild_id.to_string().as_str())
        .find(None, None)
        .await?;
    let mut saved_commands: Vec<GuildCommand> = Vec::default();

    let current_commands = guild_id.get_application_commands(&ctx.http).await?;

    while let Some(saved_command) = cursor.try_next().await? {
        saved_commands.push(saved_command);
    }

    // if there is a mismatch between the commands saved in the database vs the ones currently
    // registered with discord, clear out the guild's commands
    let commands_match = saved_commands.len() == current_commands.len()
        && current_commands
            .iter()
            .all(|current| saved_commands.iter().any(|saved| saved.eq(current)));
    if !commands_match {
        // clear guild commands
        guild_id.set_application_commands(&ctx.http, |c| c).await?;
    }

    if saved_commands.is_empty() {
        // create /setup command
        let setup_cmd = guild_id
            .create_application_command(&ctx.http, |c| {
                c.name("setup")
                    .description("Use this to register the bot's commands in this guild")
            })
            .await?;

        // save in db
        db.collection(guild_id.to_string().as_str())
            .insert_one(
                GuildCommand {
                    command_id: setup_cmd.id.0,
                    name: setup_cmd.name,
                },
                None,
            )
            .await?;
    }

    Ok(guild_id)
}
