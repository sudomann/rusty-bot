use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::utils::MessageBuilder;

use crate::DbClientRef;

/// Register a game mode
pub async fn create(ctx: &Context, interaction: &ApplicationCommandInteraction) -> String {
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let db = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };
    let mut response = MessageBuilder::new();

    // read existing game modes from db

    // if no existing game mode in db, create join command
    // Otherwise, fetch command with name: "join" from db

    // use it's id value to get a corresponding guild command via serenity

    // update that guild command's options with this game mode

    response.build()
}

/// Delete a registered game mode
pub async fn delete() {}
