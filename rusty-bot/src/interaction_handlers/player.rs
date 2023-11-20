use anyhow::Context as AnyhowContext;
use serenity::client::Context;
use serenity::model::channel::{Channel, ChannelType};
use serenity::model::interactions::application_command::ApplicationCommandInteraction;

use super::IntendedGameMode;

// !TODO: validate that this command is sent from either the registered pug channel,
// or a pug thread for an active picking session
// !TODO: lots of duplicate code in this whole module

pub async fn add_to_pug(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<crate::DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
    };
    let db = client.database(&guild_id.to_string());

    let guild_channel = match interaction
        .channel_id
        .to_channel(&ctx)
        .await
        .context("Tried to obtain `Channel` from a ChannelId")?
    {
        Channel::Guild(channel) => {
            if let ChannelType::Text = channel.kind {
                channel
            } else {
                return Ok("You cannot use this command here".to_string());
            }
        }
        _ => return Ok("You cannot use this command here".to_string()),
    };

    if interaction.data.resolved.users.len() != 1 {
        return Ok("Exactly one user must be specific by mention".to_string());
    }

    let target_user_id = interaction.data.resolved.users.keys().last().unwrap();

    let arg = &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("game_mode"))
        .context("The `game_mode` option is missing")?
        .value
        .as_ref()
        .context("The `game_mode` option does not have a value")?
        .as_str()
        .context("Somehow, the value of the `game_mode` option is not a string")?;

    let game_mode_arg = arg.to_string();

    super::queue::join_helper(
        &ctx,
        guild_id,
        guild_channel,
        db,
        IntendedGameMode::Single(game_mode_arg),
        target_user_id.0,
    )
    .await
}

pub async fn remove_from_pug(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<crate::DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
    };
    let db = client.database(&guild_id.to_string());

    let guild_channel = match interaction
        .channel_id
        .to_channel(&ctx)
        .await
        .context("Tried to obtain `Channel` from a ChannelId")?
    {
        Channel::Guild(channel) => {
            if let ChannelType::Text = channel.kind {
                channel
            } else {
                return Ok("You cannot use this command here".to_string());
            }
        }
        _ => return Ok("You cannot use this command here".to_string()),
    };

    if interaction.data.resolved.users.len() != 1 {
        return Ok("Exactly one user must be specific by mention".to_string());
    }

    let target_user_id = interaction.data.resolved.users.keys().last().unwrap();

    let arg = &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("game_mode"))
        .context("The `game_mode` option is missing")?
        .value
        .as_ref()
        .context("The `game_mode` option does not have a value")?
        .as_str()
        .context("Somehow, the value of the `game_mode` option is not a string")?;

    let game_mode_arg = arg.to_string();

    super::queue::leave_helper(
        &ctx,
        guild_id,
        guild_channel,
        db,
        IntendedGameMode::Single(game_mode_arg),
        target_user_id.0,
    )
    .await
}
