use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::error::Error;
use mongodb::options::FindOneOptions;
use mongodb::Database;

use crate::db::collection_name::PLAYER_ROSTER;

use super::collection_name::{COMMANDS, GAME_MODES, GAME_MODE_JOINS, PICKING_SESSIONS};
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

pub async fn get_current_picking_session(db: Database) -> Result<Option<PickingSession>, Error> {
    // !FIXME: which direction does this sort go in
    let options = FindOneOptions::builder()
        .sort(doc! { "created": 1 })
        .build();
    db.collection(PICKING_SESSIONS)
        .find_one(None, options)
        .await
}

pub async fn is_captain_position_available(
    db: Database,
    &pug_thread_channel_id: &u64,
) -> Result<bool, Error> {
    let collection = db.collection::<Player>(PLAYER_ROSTER);
    let filter = doc! {
        "channel_id_for_picking_session": pug_thread_channel_id as i64,
        "is_captain": true,
    };

    let current_captain_count = collection.count_documents(filter, None).await?;
    if current_captain_count < 2 {
        return Ok(true);
    }
    Ok(false)
}

pub async fn get_picking_session_members(
    db: Database,
    &pug_thread_channel_id: &u64,
) -> Result<Vec<Player>, Error> {
    let collection = db.collection::<Player>(PLAYER_ROSTER);
    let filter = doc! {
        "channel_id_for_picking_session": pug_thread_channel_id as i64,
    };

    let cursor = collection.find(filter, None).await?;
    cursor.try_collect().await
}

/// Fetch guild command records for the following guild commands:
///
/// - **/captain**
/// - **/nocapt**
/// - **/autocaptain**
pub async fn get_captain_related_guild_commands(db: Database) -> Result<Vec<GuildCommand>, Error> {
    let collection = db.collection::<GuildCommand>(COMMANDS);
    let filter = doc! {
        "name": {
            "$in": ["captain", "nocapt", "autocaptain"]
        },
    };

    let cursor = collection.find(filter, None).await?;
    cursor.try_collect().await
}
