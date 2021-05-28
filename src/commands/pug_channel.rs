use crate::data_structure::DesignatedPugChannel;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

#[command("pugchannel")]
/// Type this command in a channel to set it as the designated pug channel.
///
/// You can only designate one pug channel at a time. If there's already one, it gets replaced with the one you mention.
#[sub_commands(pug_channel_unset)]
async fn pug_channel_set(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let data_read = ctx.data.read().await;
    let lock_for_designated_pug_channel = data_read
        .get::<DesignatedPugChannel>()
        .expect("Expected DesignatedPugChannel in TypeMap");
    let mut designated_pugs_channel = lock_for_designated_pug_channel.write().await;
    designated_pugs_channel.insert(guild_id, msg.channel_id);
    let response = MessageBuilder::new()
        .push("This channel is now the designated pug channel")
        .build();
    let _ = msg.reply(ctx, response).await;

    Ok(())
}

#[command("unset")]
async fn pug_channel_unset(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let data_read = ctx.data.read().await;
    let lock_for_designated_pug_channel = data_read
        .get::<DesignatedPugChannel>()
        .expect("Expected DesignatedPugChannel in TypeMap");
    let mut designated_pugs_channel = lock_for_designated_pug_channel.write().await;
    match designated_pugs_channel.remove(&guild_id) {
        Some(channel_id) => {
            let _ = msg
                .reply(
                    ctx,
                    format!(
                        "{} is no longer the designated pug channel",
                        channel_id.mention()
                    ),
                )
                .await;
        }
        None => {
            let _ = msg
                .reply(
                    ctx,
                    "There is no designated pug channel which you can unset",
                )
                .await;
        }
    }

    Ok(())
}
