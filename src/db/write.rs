use mongodb::bson::doc;
use mongodb::error::Error;
use mongodb::results::{InsertOneResult, UpdateResult};
use mongodb::Database;
use serenity::model::id::{ChannelId, GuildId};

use super::model::*;

/// The "policy" or "method" to use when writing to the database.
pub enum Method {
    /// If a document in a given collection exists with a matching
    /// guild id, it is replaced with the incoming one.
    REPLACE,
    /// If a document in a given collection exists with a matching
    /// guild id, it is left in place and the incoming one added.
    INSERT,
}

// can these be combined with the picking_session module?
pub async fn write_new_game_mode() -> Result<(), ()> {
    Ok(())
}

pub async fn delete_game_mode() -> Result<(), ()> {
    Ok(())
}

pub async fn insert_player_in_pug() -> Result<(), ()> {
    Ok(())
}

pub async fn remove_player_from_pug() -> Result<(), ()> {
    Ok(())
}

pub async fn pick_player_for_pug_team() -> Result<(), ()> {
    Ok(())
}

pub async fn reset_pug() -> Result<(), ()> {
    Ok(())
}

pub async fn set_pug_captain() -> Result<(), ()> {
    Ok(())
}

pub async fn exclude_player_from_random_captaining() -> Result<(), ()> {
    Ok(())
}

pub async fn register_guild(db: Database, guild_id: &GuildId) -> Result<InsertOneResult, Error> {
    let collection = db.collection::<Guild>("guilds");
    let new_guild = Guild {
        guild_id: *guild_id.as_u64(),
        disabled: false,
    };
    Ok(collection.insert_one(new_guild, None).await?)
}

pub async fn set_pug_channel(
    db: Database,
    guild_id: &GuildId,
    channel_id: &ChannelId,
) -> Result<UpdateResult, Error> {
    let collection = db.collection::<PugChannel>("pug_channels");

    let desired_pug_channel = PugChannel {
        guild_id: *guild_id.as_u64(),
        channel_id: *channel_id.as_u64(),
        name: None,
    };

    let query = doc! {
        "guild_id": *guild_id.as_u64() as i64,
    };

    Ok(collection
        .replace_one(query, desired_pug_channel, None)
        .await?)
}
