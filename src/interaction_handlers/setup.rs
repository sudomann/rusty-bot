use futures::future::join_all;
use futures::{try_join, TryFutureExt};
use nanoid::nanoid;
use serenity::builder::CreateApplicationCommand;
use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::utils::MessageBuilder;
use tokio::spawn;
use tracing::error;

use crate::command_builder::base::*;
use crate::db::model::GuildCommand;
use crate::db::write::{clear_guild_commands, save_guild_commands};
use crate::DbClientRef;

/// Composes and applies base command set for a guild.
/// TODO: Checks to ensure that caller has bot admin role
/// then kicks off creation of guild command set (overwriting any existing).
///
/// The database is checked for existing data
/// such as game modes, so the commands created can be customized for the guild.
pub async fn set_guild_base_command_set(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    // FIXME: replace the verbose error handling with anyhow sugar
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let mut response = MessageBuilder::new();

    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };
    let db = client.database(guild_id.0.to_string().as_str());

    let mut command_set: Vec<CreateApplicationCommand> = Vec::default();
    let game_modes = crate::db::read::get_game_modes(db.clone()).await?;
    let mut builders = vec![
        spawn(build_pugchannel(db.clone())),
        spawn(build_addmod(db.clone())),
        spawn(build_delmod(db.clone())),
        spawn(build_last(db.clone())),
    ];
    if !game_modes.is_empty() {
        // the following commands are only useable if game modes exist
        builders.extend(vec![
            spawn(build_join(db.clone(), Some(game_modes.clone()))),
            // spawn(build_leave(db.clone(), &game_modes)),
            // ...
        ]);
    }
    // spawn all command builders
    for handle in join_all(builders).await {
        match handle {
            Ok(result) => match result {
                Ok(command_creation) => {
                    command_set.push(command_creation);
                }
                Err(err) => {
                    let id = nanoid!(6);
                    error!("Error [{}] panic in a command builder: {:?}", id, err);
                    response
                        .push("Sorry, an error occured when communicating with the database. Incident ID: ")
                        .push(id);
                    return Ok(response.build());
                }
            },
            Err(err) => {
                if err.is_panic() {
                    let id = nanoid!(6);
                    error!("Error [{}] panic in a command builder: {:?}", id, err);
                    response
                        .push("Sorry, an error occured. Incident ID: ")
                        .push(id);
                    return Ok(response.build());
                }
            }
        }
    }

    // set (overwrite) current guild commands with the built set
    match guild_id
        .set_application_commands(&ctx.http, move |c| {
            for command in command_set.into_iter() {
                c.add_application_command(command);
            }
            c
        })
        .await
    {
        Ok(created_commands) => {
            let commands_to_save: Vec<GuildCommand> = created_commands
                .iter()
                .map(|c| GuildCommand {
                    command_id: c.id.0,
                    name: c.name.clone(),
                })
                .collect();

            let clearing_fut = clear_guild_commands(db.clone())
                .map_err(|e| format!("Unable to clear {:?} commands: {:#?}", guild_id, e));

            let saving_fut = save_guild_commands(db, commands_to_save).map_err(|e| {
                format!(
                    "Cleared {:?} commands, but was unable to save created base command set: {:#?}",
                    guild_id, e
                )
            });

            match try_join!(clearing_fut, saving_fut) {
                Ok(_r) => {
                    return Ok(response.push("All done").build());
                }
                Err(err) => {
                    let id = nanoid!(6);
                    error!("Error [{}] updating command documents: {:?}", id, err);
                    return Ok(response
                    .push_line("Commands have been set, but something went wrong recording the changes.")
                    .push_line(
                        "A future launch/startup of the bot will result in \
                        my commands in this servers being cleared/reset.",
                    )
                    .push("Incident ID: ")
                    .push(id)
                    .build());
                }
            };
        }
        Err(err) => {
            let id = nanoid!(6);
            error!("Error [{}] when setting guild commands: {:?}", id, err);
            response
                .push_line("Something went wrong when setting commands for this server.")
                .push("Incident ID: ")
                .push(id);
            return Ok(response.build());
        }
    };
}
