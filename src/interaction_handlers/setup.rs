use futures::future::join_all;
use mongodb::Database;
use nanoid::nanoid;
use serenity::builder::CreateApplicationCommand;
use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::utils::MessageBuilder;
use tokio::spawn;
use tracing::error;

use crate::DbClientRef;

/// Creates base guild command set.
/// Checks to ensure that caller has bot admin role, then kicks off creation of guild command set (overwrites any existing).
///
/// The database is checked for existing data
/// such as game modes, so the commands created can be customized for the guild.
pub async fn create_guild_commands(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> String {
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };
    let guild_id = interaction.guild_id.unwrap();
    let db = client.database(guild_id.0.to_string().as_str());
    let mut response = MessageBuilder::new();

    let mut command_set: Vec<CreateApplicationCommand> = Vec::default();
    // spawn all command builders
    for handle in join_all(vec![
        spawn(build_pugchannel(db.clone())),
        spawn(build_addmod(db.clone())),
        spawn(build_delmod(db.clone())),
        spawn(build_last(db.clone())),
    ])
    .await
    {
        match handle {
            Ok(result) => match result {
                Ok(command_creation) => {
                    command_set.push(command_creation);
                }
                Err(_err) => {
                    // TODO: implement a response here
                }
            },
            Err(err) => {
                if err.is_panic() {
                    let id = nanoid!(6);
                    error!("Error [{}] during command building: {:?}", id, err);
                    response
                        .push("Sorry, an error occured. Incident ID: ")
                        .push(id);
                    return response.build();
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
        Ok(_created_commands) => {
            return response.push("All done").build();
        }
        Err(err) => {
            let id = nanoid!(6);
            error!("Error [{}] when overwriting guild commands: {:?}", id, err);
            response
                .push("Sorry, an error occured. Incident ID: ")
                .push(id);
            return response.build();
        }
    };
}

// -----------------
// Base command set
// -----------------

pub async fn build_pugchannel(
    db: Database,
) -> Result<CreateApplicationCommand, mongodb::error::Error> {
    Ok(CreateApplicationCommand::default())
}
pub async fn build_addmod(db: Database) -> Result<CreateApplicationCommand, mongodb::error::Error> {
    // read db for 2 commands, "addmod" and "join"

    // use the id to recover the corresponding command in serenity

    // addmod command options:
    // - game name field
    // - player count choices field
    //    - allowed value range: 2-24 (even only)

    // after addmod is used, we edit the join command
    Ok(CreateApplicationCommand::default())
}
pub async fn build_delmod(db: Database) -> Result<CreateApplicationCommand, mongodb::error::Error> {
    Ok(CreateApplicationCommand::default())
}
pub async fn build_last(db: Database) -> Result<CreateApplicationCommand, mongodb::error::Error> {
    Ok(CreateApplicationCommand::default())
}
