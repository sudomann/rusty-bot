use nanoid::nanoid;
use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::utils::MessageBuilder;
use tracing::error;

use crate::db::write::set_pug_channel;
use crate::DbRef;

/// Declare a text channel in a guild as the designated pug channel
pub async fn set(ctx: &Context, interaction: &ApplicationCommandInteraction) -> String {
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let db = {
        let data = ctx.data.read().await;
        data.get::<DbRef>().unwrap().clone()
    };
    let mut response = MessageBuilder::new();

    match set_pug_channel(db, &interaction.guild_id.unwrap(), &interaction.channel_id).await {
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
