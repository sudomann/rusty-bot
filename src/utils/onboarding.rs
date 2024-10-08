use futures::future::join_all;
use itertools::Itertools;
use mongodb::Client;
use std::sync::Arc;

use serenity::client::Context;
use serenity::model::id::{CommandId, GuildId};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tracing::{error, info, instrument, warn};

use crate::db::model::GuildCommand;
use crate::db::write::clear_guild_commands;
use crate::DbClientRef;

/// For each guild, check for presence of guild application commands created by this bot.
/// If there aren't suitable existing commands, create a `/help` command
#[instrument(skip(ctx, guild_ids))]
pub async fn inspect_guild_commands(ctx: Arc<Context>, guild_ids: Vec<GuildId>) {
    let mut interval = interval(Duration::from_secs(5));

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
    let mut ordered_guild_names: Vec<String> = Vec::default();

    info!("Launching one task per connected guild for conducting inspection");
    for guild_id in guild_ids {
        let guild_name = match guild_id.to_guild_cached(&ctx.cache) {
            Some(guild) => guild.name.clone(),
            None => "<guild_name_unavailable>".to_string(),
        };
        join_handles.push(tokio::spawn(inspect_and_maybe_update_db(
            ctx.clone(),
            guild_id,
            db_client.clone(),
        )));
        ordered_guild_names.push(guild_name);
    }

    for (i, handle) in join_all(join_handles).await.iter().enumerate() {
        match handle {
            Ok(result) => {
                if let Err(err) = result {
                    error!("{}\n{:?}", ordered_guild_names.get(i).unwrap(), err);
                }
            }
            Err(join_err) => {
                error!("Failed to rejoin a task:\n {:?}", join_err);
            }
        }
    }
    info!("Inspections complete!");
}

/// Register /help command as necessary for guilds
///
/// TODO: ensure that guilds marked as disabled don't have/get any guild commands registered.
pub async fn inspect_and_maybe_update_db(
    ctx: Arc<Context>,
    guild_id: GuildId,
    db_client: Client,
) -> Result<GuildId, crate::error::Error> {
    let db = db_client.database(&guild_id.to_string());

    let current_commands = guild_id.get_commands(&ctx.http).await?;
    let mut saved_commands: Vec<GuildCommand> = crate::db::read::get_commands(db.clone()).await?;

    // if there is a mismatch between the commands saved in the database vs the ones currently
    // registered with discord, clear out the guild's commands
    // We do this because it suggests the arrangement of registered commands in the database
    // has grown apart from what the code expects.
    // Thus the code is likely faulty and should not be allowed to quietly continue corrupting data
    // !FIXME: the bot saves duplicate records of the same command for some reason

    let commands_match = saved_commands.len() == current_commands.len()
        && current_commands.iter().all(|current| {
            saved_commands
                .iter()
                .any(|saved| saved.command_id as u64 == current.id.get())
        });

    if !commands_match {
        let a_c = current_commands
            .iter()
            .format_with(", ", |cmd, f| f(&format_args!("{} {}", cmd.name, cmd.id)));
        let s_c = saved_commands.iter().format_with(", ", |cmd, f| {
            f(&format_args!(
                "{} {}",
                cmd.name,
                CommandId::from(cmd.command_id as u64)
            ))
        });
        let output = format!(
            "Mismatch in command set for {:?}\n\
            Current Guild Application Commands: {}\n\
            Commands Saved in DB: {}\n\
            Clearing all existing commands from guild and database...",
            &guild_id, a_c, s_c
        );
        warn!("{}", output);
        // clear guild commands
        guild_id.set_commands(&ctx.http, Vec::new()).await?;
        // clear db also
        clear_guild_commands(db.clone()).await?;
        // and empty the vec that might contain old results
        // from the db which we just ^ cleared
        saved_commands.clear();
    }

    if saved_commands.is_empty() {
        // create /help command

        let help_cmd = guild_id
            .create_command(&ctx.http, crate::command_builder::base::build_help())
            .await?;

        // save in db
        crate::db::write::register_guild_command(db, &help_cmd).await?;
    }

    Ok(guild_id)
}
