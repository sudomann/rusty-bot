use anyhow::{bail, Context as AnyhowContext};
use mongodb::Database;
use serenity::client::Context;
use serenity::model::id::{CommandId, GuildId};

use crate::command_builder::base::{
    build_addplayer, build_delmod, build_delplayer, build_join, build_last, build_leave,
};
use crate::db;
use crate::db::model::GameMode;

const COMMANDS_WITH_GAME_MODE_OPTION: &[&str; 6] =
    &["join", "leave", "delmod", "last", "addplayer", "delplayer"];

/// The commands listed require an up-to-date list of game modes to display as choices -
/// this is a convenience function to update them for a given guild, provided a list of game modes:
///
/// - /join
/// - /leave
/// - /delmod
/// - /last
/// - /addplayer
/// - /delplayer
pub async fn refresh_commands_with_game_mode_option(
    ctx: &Context,
    guild_id: GuildId,
    db: Database,
    game_modes: Vec<GameMode>,
) -> anyhow::Result<()> {
    // !TODO: current implementation is tooo slow
    // consider using tokio::spawn + join_all to parallelize, so it completes under 3 secs
    for command_name in COMMANDS_WITH_GAME_MODE_OPTION {
        let saved_guild_command = db::read::find_command(db.clone(), &command_name)
            .await?
            .context(format!(
                "No `{}` command was found in the database",
                &command_name
            ))?;

        let updated_command_to_apply = match *command_name {
            "join" => build_join(&game_modes),
            "leave" => build_leave(&game_modes),
            "delmod" => build_delmod(&game_modes),
            "last" => build_last(&game_modes),
            "addplayer" => build_addplayer(&game_modes),
            "delplayer" => build_delplayer(&game_modes),
            _ => {
                bail!("Double-check match arms against command set for a typo in command name");
            }
        };

        guild_id
            .edit_application_command(&ctx.http, CommandId(saved_guild_command.command_id as u64), |c| {
                *c = updated_command_to_apply;
                c
            })
            .await
            .context(
                "Attempted to edit existing join application command to \
                overwrite its options with a new one which has an \
                up to date list of game mode choices",
            )?;
    }

    Ok(())
}
