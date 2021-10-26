use futures::future::join_all;
use futures::{try_join, TryFutureExt};
use mongodb::Database;
use nanoid::nanoid;
use serenity::builder::{CreateApplicationCommand, CreateApplicationCommandOption};
use serenity::client::Context;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandOptionType,
};
use serenity::utils::MessageBuilder;
use tokio::spawn;
use tracing::error;

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
) -> String {
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let mut response = MessageBuilder::new();

    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };
    let db = client.database(guild_id.0.to_string().as_str());

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
                Err(err) => {
                    let id = nanoid!(6);
                    error!("Error [{}] panic in a command builder: {:?}", id, err);
                    response
                        .push("Sorry, an error occured when communicating with the database. Incident ID: ")
                        .push(id);
                    return response.build();
                }
            },
            Err(err) => {
                if err.is_panic() {
                    let id = nanoid!(6);
                    error!("Error [{}] panic in a command builder: {:?}", id, err);
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
                    return response.push("All done").build();
                }
                Err(err) => {
                    let id = nanoid!(6);
                    error!("Error [{}] updating command documents: {:?}", id, err);
                    return response
                    .push_line("Commands have been set, but something went wrong recording the changes.")
                    .push_line(
                        "A future launch/startup of the bot will result in \
                        my commands in this servers being cleared/reset.",
                    )
                    .push("Incident ID: ")
                    .push(id)
                    .build();
                }
            };
        }
        Err(err) => {
            let id = nanoid!(6);
            error!("Error [{}] when setting guild commands: {:?}", id, err);
            response
                .push_line("Something went wrong when setting commands for this guild.")
                .push("Incident ID: ")
                .push(id);
            return response.build();
        }
    };
}

// -----------------
// Base command set
// -----------------

pub async fn build_pugchannel(
    _db: Database,
) -> Result<CreateApplicationCommand, mongodb::error::Error> {
    let mut cmd = CreateApplicationCommand::default();
    cmd.name("setpugchannel")
        .description("Designate a channel to be used for pugs");
    Ok(cmd)
}

pub async fn build_addmod(
    _db: Database,
) -> Result<CreateApplicationCommand, mongodb::error::Error> {
    let mut label_option = CreateApplicationCommandOption::default();
    label_option
        .name("label")
        .description("Name of the game mode")
        .kind(ApplicationCommandOptionType::String)
        .required(true);

    let mut player_count_option = CreateApplicationCommandOption::default();
    player_count_option
        .name("player_count")
        .description(
            "Number of players required to fill the game mode. Must be even, minimum 2, maximum 24",
        )
        .kind(ApplicationCommandOptionType::Integer)
        .add_int_choice("2", 2)
        .add_int_choice("4", 4)
        .add_int_choice("6", 6)
        .add_int_choice("8", 8)
        .add_int_choice("10", 10)
        .add_int_choice("12", 12)
        .add_int_choice("14", 14)
        .add_int_choice("16", 16)
        .add_int_choice("18", 18)
        .add_int_choice("20", 20)
        .add_int_choice("22", 22)
        .add_int_choice("24", 24)
        .required(true);

    let mut cmd = CreateApplicationCommand::default();
    cmd.name("addmod")
        .description("Add a new game mode")
        .add_option(label_option)
        .add_option(player_count_option);

    Ok(cmd)
}

pub async fn build_delmod(db: Database) -> Result<CreateApplicationCommand, mongodb::error::Error> {
    let mut game_mode_option = CreateApplicationCommandOption::default();

    // load existing game modes
    for existing_game_mode in crate::db::read::get_game_modes(db).await?.iter() {
        game_mode_option.add_string_choice(&existing_game_mode.label, &existing_game_mode.key);
    }

    game_mode_option
        .name("game_mode")
        .description("The label of the game mode you want to delete")
        .kind(ApplicationCommandOptionType::String)
        .required(true);

    let mut cmd = CreateApplicationCommand::default();
    cmd.name("delmod")
        .description("Delete an existing game mode")
        .add_option(game_mode_option);
    Ok(cmd)
}

pub async fn build_last(_db: Database) -> Result<CreateApplicationCommand, mongodb::error::Error> {
    let mut history_count_option = CreateApplicationCommandOption::default();
    history_count_option
        .name("match_age")
        .description(
            "How many steps/matches to traverse into match history when searching for a match to display",
        )
        .kind(ApplicationCommandOptionType::Integer);

    let mut cmd = CreateApplicationCommand::default();
    cmd.name("last")
        .description("Display info about a previous pug")
        .add_option(history_count_option);
    Ok(cmd)
}
