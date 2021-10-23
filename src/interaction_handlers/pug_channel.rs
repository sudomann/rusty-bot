use nanoid::nanoid;
use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::utils::MessageBuilder;
use tracing::error;

use crate::db::write::set_pug_channel;
use crate::DbClientRef;

/// Declare a text channel in a guild as the designated pug channel
pub async fn set(ctx: &Context, interaction: &ApplicationCommandInteraction) -> String {
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };

    let channel_id = interaction.channel_id;
    let guild_id = interaction.guild_id.unwrap();
    let db = client.database(guild_id.0.to_string().as_str());

    let mut response = MessageBuilder::new();

    match set_pug_channel(db, channel_id.0, channel_id.name(&ctx.cache).await).await {
        Ok(_result) => {
            response
                .mention(&interaction.channel_id)
                .push(" is now the designated pug channel");
        }
        Err(err) => {
            let id = nanoid!(6);
            error!("Error [{}] when to setting pug channel: {:?}", id, err);
            response
                .push("Sorry, an error occured. Incident ID: ")
                .push(id);
        }
    }

    response.build()
}
