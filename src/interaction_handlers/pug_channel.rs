use anyhow::Context as AnyhowContext;
use serenity::client::Context;
use serenity::model::application::CommandInteraction;
use serenity::utils::MessageBuilder;

use crate::db::write::set_pug_channel;
use crate::DbClientRef;

/// Declare a text channel in a guild as the designated pug channel
pub async fn set(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<String> {
    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
    };

    let channel_id = interaction.channel_id;
    let guild_id = interaction.guild_id.unwrap();
    let db = client.database(guild_id.get().to_string().as_str());

    let channel_name = channel_id
        .name(&ctx.http)
        .await
        .context("Failed to fetch channel name")?;

    set_pug_channel(db, channel_id.get(), Some(channel_name), Vec::new()).await?;

    let response = MessageBuilder::new()
        .mention(&channel_id)
        .push(" is now the designated pug channel")
        .build();

    Ok(response)
}
