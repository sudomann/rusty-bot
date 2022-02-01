use anyhow::Context as AnyhowContext;
use futures::try_join;
use serenity::client::Context;
use serenity::model::channel::Message;

use crate::command_builder::{base::*, *};
use crate::db::model::{PickingSession, Player};
use crate::db::write::{clear_guild_commands, save_guild_commands};
use crate::DbClientRef;

// !FIXME: This does not cover the case where there is an active picking session.
// If someone ran .configure during a picking session
// because commands became corrupted, or for whatever other reason,
// The new command set would be lacking pug/picking commands
/// Composes and applies command set for a guild.
/// TODO: Checks to ensure that caller has bot admin role
/// then kicks off creation of guild command set (overwriting all existing).
///
/// The database is checked for existing data
/// such as game modes, so the commands created can be customized for the guild.
pub async fn generate_and_apply_guild_command_set(
    ctx: &Context,
    original_msg: &Message,
) -> anyhow::Result<String> {
    let _working = original_msg.channel_id.start_typing(&ctx.http);

    let guild_id = original_msg.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };
    let db = client.database(guild_id.0.to_string().as_str());

    let game_modes = crate::db::read::get_game_modes(db.clone()).await?;

    // sequentially spawn all command builders
    // Tried to make them run in parallel by spawning async blocks containing these function calls
    // then `join_all`ing, but rust complains about the lifetime of game_modes
    let mut command_set = vec![
        build_help(),
        build_pugchannel(),
        build_addmod(),
        build_delmod(&game_modes),
        build_list(),
        build_last(&game_modes),
        build_join(&game_modes),
        build_leave(&game_modes),
        build_addplayer(&game_modes),
        build_delplayer(&game_modes),
    ];

    // check for an active picking session
    let active_picking_session: Option<PickingSession> =
        crate::db::read::get_current_picking_session(db.clone())
            .await
            .context("Tried checking for an active picking session")?;

    if let Some(picking_session) = active_picking_session {
        let players: Vec<Player> = crate::db::read::get_picking_session_members(
            db.clone(),
            &picking_session.thread_channel_id.parse::<u64>()?,
        )
        .await
        .context("Failed to obtain list of players in picking session")?;

        let players_as_users = crate::utils::transform::players_to_users(&ctx, players).await?;
        command_set.push(build_autocaptain());
        command_set.push(build_captain());
        command_set.push(build_reset());
        command_set.push(build_pick(&players_as_users));
    }

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
