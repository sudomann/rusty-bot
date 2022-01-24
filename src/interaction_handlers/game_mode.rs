use anyhow::Context as AnyhowContext;
use mongodb::bson::doc;
use serenity::client::Context;
use serenity::model::id::CommandId;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;

use crate::command_builder::base::generate_join_command_option;
use crate::db;
use crate::db::model::GameMode;
use crate::DbClientRef;

/// Register a game mode
///
/// Expects fields `label` and `player_count`
pub async fn create(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };
    let db = client.database(&guild_id.to_string());

    let label = &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("label"))
        .context("The `label` option is missing")?
        .value
        .as_ref()
        .context("The `label` option does not have a value")?
        .as_str()
        .context("Somehow, the value of the `label` option is not a string")?;

    let player_count = &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("player_count"))
        .context("The `player_count` option is missing")?
        .value
        .as_ref()
        .context("The `player_count` option does not have a value")?
        .as_u64()
        .context("Failed to extract the value of the `player_count` option as a `u64`")?;

    // read existing game modes from db
    let mut game_modes = db::read::get_game_modes(db.clone()).await?;

    // check for conflict/existing
    if game_modes.iter().any(|g| g.label.eq(label)) {
        return Ok("A game mode with this label already exists".to_string());
    }

    // save new game mode
    db::write::write_new_game_mode(db.clone(), label.to_string(), *player_count).await?;

    // try to retrieve the existing join command
    let saved_cmd = db::read::find_command(db.clone(), "join")
            .await?
            .context("At least one game mode exists in the database, but no join command was found in the database")?;
    let current_join_cmd = guild_id
        .get_application_command(&ctx.http, CommandId(saved_cmd.command_id))
        .await
        .context(
            "Attempted to fetch and application command object for `join` from discord \
                using the CommandId from a `join` command that was saved in the database",
        )?;

    /*
        Since there's no nice/practical way to edit a particular option's
        choices, we:
        - create a new option (game mode name/label) object
            which has up-to-date game mode choices
        - overwrite any existing options with this new one
    */

    // Must add the desired game mode to the list since it the list only contains
    // game modes that existed before
    game_modes.push(GameMode {
        label: label.to_string(),
        player_count: *player_count,
    });
    let new_option = generate_join_command_option(&game_modes);
    guild_id
        .edit_application_command(&ctx.http, current_join_cmd.id, |c| {
            c.set_options(vec![new_option])
        })
        .await
        .context(
            "Attempted to edit existing join application command to \
                overwrite its options with a new one which has an \
                up to date list of game mode choices",
        )?;
    // FIXME: this does not yet update everything it should,
    // e.g. game mode choices for /leave, /delmod
    // consult repo README

    Ok(format!("Added new game mode {} successfully", label))
}

/// Delete a registered game mode
pub async fn delete(
    _ctx: &Context,
    _interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    // for this game mode to be deleted:

    // if a pug is in "picking" state
    // inform caller of this as the reason it cannot be deleted

    // if players have joined the queue for it
    // instruct caller to remove them all first
    Ok("Deleted successfully".to_string())
}
