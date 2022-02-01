use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::utils::MessageBuilder;

use crate::db::write::set_pug_channel;
use crate::DbClientRef;

/// Declare a text channel in a guild as the designated pug channel
pub async fn set(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
    };

    let channel_id = interaction.channel_id;
    let guild_id = interaction.guild_id.unwrap();
    let db = client.database(guild_id.0.to_string().as_str());

    set_pug_channel(db, channel_id.0, channel_id.name(&ctx.cache).await).await?;
    let response = MessageBuilder::default()
        .mention(&interaction.channel_id)
        .push(" is now the designated pug channel")
        .build();

    Ok(response)
}
