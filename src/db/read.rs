use futures::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::error::Error;
use mongodb::Database;
use serenity::model::id::GuildId;

use super::collection_name::COMMANDS;
use super::model::*;

/// Get list of known guilds
pub async fn get_known_guilds(db: Database) -> Result<Vec<Guild>, Error> {
    let collection = db.collection::<Guild>("guilds");
    let cursor = collection.find(None, None).await?;
    Ok(cursor.try_collect().await?)
}

/// Fetch a known guild by it's guild id
pub async fn get_guild(db: Database, guild_id: &GuildId) -> Result<Option<Guild>, Error> {
    let collection = db.collection::<Guild>("guilds");
    let filter = doc! {
        "guild_id": *guild_id.as_u64() as i64,
    };
    Ok(collection.find_one(Some(filter), None).await?)
}

/// Get saved guild commands
pub async fn get_commands(db: Database) -> Result<Vec<GuildCommand>, Error> {
    let mut cursor = db
        .collection::<GuildCommand>(COMMANDS)
        .find(None, None)
        .await?;

    let mut saved_commands: Vec<GuildCommand> = Vec::default();
    while let Some(saved_command) = cursor.try_next().await? {
        saved_commands.push(saved_command);
    }
    Ok(saved_commands)
}

/// Fetch a guild's designated pug channels
pub async fn get_pug_channel(
    db: Database,
    guild_id: &GuildId,
) -> Result<Option<PugChannel>, Error> {
    let collection = db.collection::<PugChannel>("pug_channels");
    let filter = doc! {
        "guild_id": *guild_id.as_u64() as i64,
    };
    Ok(collection.find_one(Some(filter), None).await?)
}
