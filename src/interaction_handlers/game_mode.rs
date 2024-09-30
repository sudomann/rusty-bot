use anyhow::Context as AnyhowContext;
use serenity::client::Context;
use serenity::model::application::CommandInteraction;

use crate::db;
use crate::db::model::GameMode;
use crate::utils::application_commands::refresh_commands_with_game_mode_option;
use crate::DbClientRef;

/// Register a game mode
///
/// Expects fields `label` and `player_count`
pub async fn create(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<String> {
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
    };
    let db = client.database(&guild_id.to_string());

    let label = &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("label"))
        .context("The `label` option is missing")?
        .value
        .as_str()
        .context("Somehow, the value of the `label` option is not a string")?;

    let player_count = &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("player_count"))
        .context("The `player_count` option is missing")?
        .value
        .as_i64()
        .context("Failed to extract the value of the `player_count` option as a `i64`")?;

    // read existing game modes from db
    let mut game_modes = db::read::get_game_modes(db.clone()).await?;

    // check for conflict/existing
    if game_modes.iter().any(|g| g.label.eq(label)) {
        return Ok("A game mode with this label already exists".to_string());
    }

    // save new game mode
    db::write::write_new_game_mode(db.clone(), label.to_string(), *player_count as u64).await?;

    // Must add the desired game mode to the list since it the list only contains
    // game modes that existed before
    game_modes.push(GameMode {
        label: label.to_string(),
        player_count: *player_count as i64,
    });

    // Finally, update commands which require an up-to-date game mode list
    refresh_commands_with_game_mode_option(&ctx, guild_id, db, game_modes)
        .await
        .context(
            "Attempted to update relevant commands with an \
            up to date list of game mode choices",
        )?;

    Ok(format!("Added new game mode {} successfully", label))
}

/// Delete a registered game mode.
///
/// This updates the set of commands which require an up-to-date list of game modes to show as choices.
pub async fn delete(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<String> {
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
    };
    let db = client.database(&guild_id.to_string());

    let game_mode_label = &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("game_mode"))
        .context("The `game_mode` option is missing")?
        .value
        .as_str()
        .context("Somehow, the value of the `game_mode` option is not a string")?;

    // read existing game modes from db
    let mut game_modes = db::read::get_game_modes(db.clone()).await?;
    // Remove matching command
    let match_position = match game_modes.iter().position(|g| g.label.eq(game_mode_label)) {
        Some(position) => position,
        None => {
            return Ok(format!(
                "No game mode called **{}** was found",
                game_mode_label
            ))
        }
    };
    game_modes.remove(match_position);

    // if the queue for the game mode is not empty,
    // instruct caller to remove all queued players,
    // then try to delete again
    let queue = db::read::get_game_mode_queue(db.clone(), &game_mode_label.to_string()).await?;
    if !queue.is_empty() {
        return Ok(format!(
            "The queue for **{}** is not empty. Remove any players who joined and try again.",
            game_mode_label
        ));
    }

    // if picking is in progess instruct caller
    // to wait till picking is over before deleting the game made
    if db::read::get_current_picking_session(db.clone())
        .await?
        .is_some()
    {
        return Ok(format!(
            "A picking session for **{}** is currently in progress. Try again after picking is complete.",
            game_mode_label
        ));
    };

    // Remove game mode's record from db
    let result = db::write::delete_game_mode(db.clone(), game_mode_label.to_string()).await?;
    match result.deleted_count {
        0 => anyhow::bail!("Unable to delete the {} game mode", game_mode_label),
        1 => {
            // Update game mode choices
            refresh_commands_with_game_mode_option(&ctx, guild_id, db, game_modes)
                .await
                .context(
                    "Attempted to update relevant commands with an \
                    up to date list of game mode choices",
                )?;
        }
        _ => anyhow::bail!(
            "DB function to delete a game mode seems buggy - it deleted more than 1 record."
        ),
    }

    Ok(format!("Deleted **{}** successfully", game_mode_label))
}
