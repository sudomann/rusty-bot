use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::channel::{GuildChannel, Message},
    prelude::*,
    utils::MessageBuilder,
};

use crate::data_structure::DesignatedPugChannel;

#[check]
#[name = "PugChannel"]
#[display_in_help(false)]
async fn is_pug_channel_check(
    context: &Context,
    msg: &Message,
    _: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    if let Some(guild_id) = msg.guild_id {
        // get the channel this message came from
        let current_channel: GuildChannel = match msg.channel_id.to_channel(&context).await {
            Ok(channel) => channel.guild().unwrap(),
            // TODO: remove panic - this is probably be salvageable
            Err(_why) => panic!("Failed to determine channel of message"),
        };

        // TODO: try to narrow this body as much as possible
        // to reduce time spent holding the RwLock in read mode
        let fail: Option<String> = {
            let data_read = context.data.read().await;
            // Then we obtain the value we need from data, in this case, we want the desginated pug channels.
            // The returned value from get() is an Arc, so the reference will be cloned, rather than
            // the data.
            let pug_channels_lock = data_read
                .get::<DesignatedPugChannel>()
                .expect("Expected DesignatedPugChannel in TypeMap")
                .clone();
            let pug_channels = pug_channels_lock.read().await;

            // Then use the designated pug channel of the guild this message came from
            // This time, the value is not Arc, so the data will be cloned.
            match pug_channels.get(&guild_id) {
                Some(pug_channel_id) => {
                    if current_channel.id != *pug_channel_id {
                        Some(MessageBuilder::new()
                            .push("Please go to the ")
                            .mention(pug_channel_id)
                            .push(" channel to use this command")
                            .build())
                    }
                    else {None}
                },
                None => {
                    Some(MessageBuilder::new()
                    .push("No pug channel set. ")
                    .push("Contact admins to type `.pugchannel` in the channel destined for pugs.")
                    .build())
                },
            }
        };
        if let Some(response) = fail {
            Err(Reason::User(response))
        } else {
            Ok(())
        }
    } else {
        panic!("No GuildId in received message - Is client running without gateway?");
    }
}
