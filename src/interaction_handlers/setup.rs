use anyhow::Context as AnyhowContext;
use futures::future::join_all;
use futures::try_join;
use serenity::builder::CreateApplicationCommand;
use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use tokio::spawn;

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
    let _working = interaction.channel_id.start_typing(&ctx.http);

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
        let join_result = handle.context("A command builder panicked")?;
        let built_command = join_result.context(
            "An error occured during command building. \
            It likely came from an attempt to communicate with the database",
        )?;
        command_set.push(built_command);
    }

    // set (overwrite) current guild commands with the built set
    let created_commands = guild_id
        .set_application_commands(&ctx.http, move |c| {
            for command in command_set.into_iter() {
                c.add_application_command(command);
            }
            c
        })
        .await
        .context(format!(
            "Failed to overwrite guild commands for: {:?}",
            &guild_id
        ))?;

    let clearing_fut = clear_guild_commands(db.clone());
    let saving_fut = save_guild_commands(db, created_commands);
    try_join!(clearing_fut, saving_fut).context("Guild commands have been set, but something went wrong updating command records in the database")?;
    Ok("All done".to_string())
}
