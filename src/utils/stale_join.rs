use crate::data_structure::{DesignatedPugChannel, PugsWaitingToFill};
use serenity::{
    client::Context,
    // model::id::{GuildId, UserId},
    prelude::Mentionable,
    utils::MessageBuilder,
};
use tracing::info;
// use std::collections::HashMap;

/// The amount of time to elapse before removing players who have been in an unfilled pug too long.
const SIX_HOURS: i64 = 21600;

pub async fn remove_expired_players(ctx: &Context) {
    let data = ctx.data.read().await;
    info!("Performing `remove_expired_players()` task");

    let designated_pug_channels = {
        let lock_for_designated_pug_channel = data
            .get::<DesignatedPugChannel>()
            .expect("Expected DesignatedPugChannel in TypeMap");
        lock_for_designated_pug_channel.read().await.clone()
    };

    for (guild_id, channel_id) in designated_pug_channels.iter() {
        let lock_for_pugs_waiting_to_fill = data
            .get::<PugsWaitingToFill>()
            .expect("Expected PugsWaitingToFill in TypeMap");
        let mut pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.write().await;
        let pugs = pugs_waiting_to_fill.get_mut(guild_id).unwrap();
        let mut message = MessageBuilder::new();
        // TODO: use HashMap<UserId, Vec<GameMode>> to store removals for a guild
        // Then send a single message, one line per user which has all their kicked game modes
        // TODO: if game modes with MANY players are created and filled in future,
        // it can be required to split the message to avoid hitting discord's message size limit,
        // where the are many expired players in a pug
        for (game_mode, players) in pugs.iter_mut() {
            for player in players.clone().iter() {
                if player.time_elapsed_since_join().num_minutes() > SIX_HOURS {
                    message.push_line(
                    format!("{} - you were removed from {} because it's been six hours since you joined",
                    player.get_user_data().mention(),
                    game_mode.label())
                    );
                    players.remove(player);
                }
            }
        }
        let _ = channel_id.say(&ctx.http, message).await;
    }
}
