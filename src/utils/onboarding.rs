use futures::future::join_all;
use mongodb::Database;
use serenity::http::Http;
use std::sync::Arc;

use mongodb::error::Error;
use serenity::client::Context;
use serenity::model::id::GuildId;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument};

use crate::db::read::get_guild;
use crate::db::write::register_guild;
use crate::DbRef;

use super::create_commands::construct_guild_commands;

/// Check whether provideds guilds are saved in database. For those that are not,
/// create a new document for them in database, then create the initial guild commands.
#[instrument(skip(ctx, guild_ids))]
pub async fn ensure_guild_registration(
    ctx: Arc<Context>,
    guild_ids: Vec<GuildId>,
) -> Result<(), Error> {
    let db = {
        let data = ctx.data.read().await;
        data.get::<DbRef>().unwrap().clone()
    };

    let mut join_handles: Vec<JoinHandle<Result<GuildId, Error>>> = Vec::default();

    info!("Launching one task per connected guild for conducting inspection");
    for guild_id in guild_ids {
        join_handles.push(tokio::spawn(inspect_and_maybe_update_db(
            ctx.http.clone(),
            guild_id,
            db.clone(),
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

    Ok(())
}

/// Checks whether the `guild_id` provided is known (previously saved to database) and whether
/// it is marked as "disabled" in the database.
///
/// When known, we ensure that guilds marked as disabled don't have any guild commands registered.
///
/// When unknown, we register the guild and create guild commands for it.
async fn inspect_and_maybe_update_db(
    http: Arc<Http>,
    guild_id: GuildId,
    db: Database,
) -> Result<GuildId, Error> {
    // FIXME: irresponsible unwrap
    // If guild is in database
    if let Some(known_guild) = get_guild(db.clone(), &guild_id).await.unwrap() {
        if known_guild.disabled {
            // remove commands
            match &guild_id.get_application_commands(&http).await {
                Ok(commands) => {
                    for command in commands.iter() {
                        // TODO: how to handle failure when deleting commands?
                        let _ = guild_id.delete_application_command(&http, command.id).await;
                    }
                }
                Err(err) => {
                    error!("Serenity could not fetch guild slash commands: {:?}", err)
                }
            };
            return Ok(guild_id);
        }
    } else {
        // Guild is NOT in database so add it.
        // Unknown guilds connected to the bot at startup are by default added to the database as enabled
        register_guild(db, &guild_id).await?;
    };
    // Create the guild command set (the function that does this should be aware of registered gamemodes)
    if let Err(err) = construct_guild_commands(http, &guild_id).await {
        error!(
            "Failed to create guild commands for {}: {:?}",
            &guild_id, err
        );
    };
    // FIXME: ^ for error above, return an Err Result instead of just logging it
    Ok(guild_id)
}
