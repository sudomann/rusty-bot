use futures::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::error::Error;
use mongodb::Database;

use super::collection_name::{COMMANDS, GAME_MODES};
use super::model::*;

/// Get added game modes
pub async fn get_game_modes(db: Database) -> Result<Vec<GameMode>, Error> {
    let collection = db.collection::<GameMode>(GAME_MODES);
    let all = doc! {};
    let cursor = collection.find(all, None).await?;
    Ok(cursor.try_collect().await?)
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
