use anyhow::Context as AnyhowContext;
use futures::try_join;
use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;

use crate::command_builder::base::*;
use crate::db::write::{clear_guild_commands, save_guild_commands};
use crate::DbClientRef;

/// Composes and applies command set for a guild.
/// TODO: Checks to ensure that caller has bot admin role
/// then kicks off creation of guild command set (overwriting all existing).
///
/// The database is checked for existing data
/// such as game modes, so the commands created can be customized for the guild.
pub async fn generate_and_apply_guild_command_set(
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

    let game_modes = crate::db::read::get_game_modes(db.clone()).await?;

    // sequentially spawn all command builders
    // Tried to make them run in parallel by spawning async blocks containing these function calls
    // then `join_all`ing, but rust complains about the lifetime of game_modes
    let command_set = vec![
        build_pugchannel(),
        build_addmod(),
        build_delmod(&game_modes),
        build_last(),
        build_join(&game_modes),
        build_addplayer(&game_modes),
        build_delplayer(&game_modes),
        // spawn(build_leave(db.clone(), &game_modes)),
        // ...
    ];

    // set (overwrite) current guild commands with the newly built set
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
