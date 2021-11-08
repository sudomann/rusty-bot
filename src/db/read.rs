use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::error::Error;
use mongodb::Database;

use super::collection_name::{COMMANDS, GAME_MODES, GAME_MODE_JOINS};
use super::model::*;

/// Get added game modes
pub async fn get_game_modes(db: Database) -> Result<Vec<GameMode>, Error> {
    let collection = db.collection::<GameMode>(GAME_MODES);
    let cursor = collection.find(None, None).await?;
    cursor.try_collect().await
}

pub async fn find_game_mode(
    db: Database,
    game_mode_label: &String,
) -> Result<Option<GameMode>, Error> {
    let filter = doc! {
        "game_mode_label": game_mode_label
    };
    db.collection(GAME_MODES).find_one(filter, None).await
}

/// Get players in the waiting queue for a game mode
pub async fn get_game_mode_queue(
    db: Database,
    game_mode_label: &String,
) -> Result<Vec<GameModeJoin>, Error> {
    let collection = db.collection::<GameModeJoin>(GAME_MODE_JOINS);
    let game_mode = doc! {
        "game_mode_label": game_mode_label
    };
    let cursor = collection.find(game_mode, None).await?;
    cursor.try_collect().await
}

/// Get saved guild commands
pub async fn get_commands(db: Database) -> Result<Vec<GuildCommand>, Error> {
    let collection = db.collection::<GuildCommand>(COMMANDS);
    let cursor = collection.find(None, None).await?;

    cursor.try_collect().await
}

/// Get a single guild command matching the provided filter.
pub async fn find_command(db: Database, filter: Document) -> Result<Option<GuildCommand>, Error> {
    db.collection(COMMANDS).find_one(filter, None).await
}
