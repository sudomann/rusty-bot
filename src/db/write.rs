use chrono::Utc;
use mongodb::bson::{doc, Bson};
use mongodb::error::Error;
use mongodb::options::{FindOneAndReplaceOptions, FindOneAndUpdateOptions};
use mongodb::results::{DeleteResult, InsertManyResult, InsertOneResult, UpdateResult};
use mongodb::Database;
use serenity::model::interactions::application_command::ApplicationCommand;

use crate::db::collection_name::PLAYER_ROSTER;

use super::collection_name::{
    COMMANDS, COMPLETED_PUGS, GAME_MODES, GAME_MODE_JOINS, PICKING_SESSIONS, PUG_CHANNELS,
};
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

pub async fn write_new_game_mode(
    db: Database,
    label: String,
    player_count: u64,
) -> Result<InsertOneResult, Error> {
    let collection = db.collection(GAME_MODES);
    let game_mode = GameMode {
        label,
        player_count,
    };
    collection.insert_one(game_mode, None).await
}

pub async fn delete_game_mode() -> Result<(), ()> {
    Ok(())
}

/// Add player to queue of a game mode. This can be used repeatedly without
/// creating duplicate join records. If the user is already in the queue, the
/// join timestamp is merely updated.
pub async fn add_player_to_game_mode_queue(
    db: Database,
    game_mode_label: &String,
    player_user_id: &u64,
) -> Result<Option<GameModeJoin>, Error> {
    let collection = db.collection(GAME_MODE_JOINS);
    let filter = doc! {
        "game_mode_label": game_mode_label.clone(),
        "player_user_id": player_user_id.clone() as i64,
    };
    let join_record = GameModeJoin {
        game_mode_label: game_mode_label.clone(),
        player_user_id: player_user_id.clone(),
        joined: Utc::now(),
    };
    // create document if no existing
    let options = FindOneAndReplaceOptions::builder().upsert(true).build();
    collection
        .find_one_and_replace(filter, join_record, options)
        .await
}

pub async fn remove_player_from_game_mode_queue(
    db: Database,
    game_mode_label: String,
    player_user_id: u64,
) -> Result<Option<GameModeJoin>, Error> {
    let collection = db.collection(GAME_MODE_JOINS);
    let filter = doc! {
        "game_mode_label": game_mode_label,
        "player_user_id": player_user_id as i64
    };
    collection.find_one_and_delete(filter, None).await
}

/// Remove players from the queue of the specified game mode and put them on
/// a roster for a picking session.
/// Players are also removed from the queue of any other game mode they had joined.
///
/// Uses mongodb session feature for atomicity.
pub async fn create_picking_session(
    db: Database,
    pug_thread_channel_id: &u64,
    game_mode_label: &String,
    players: &Vec<u64>,
    pick_sequence: Vec<Team>,
) -> Result<InsertOneResult, Error> {
    // FIXME: use session for atomicity!
    let game_mode_join_collection = db.collection::<GameModeJoin>(GAME_MODE_JOINS);
    let picking_session_collection = db.collection(PICKING_SESSIONS);
    let player_roster_collection = db.collection(PLAYER_ROSTER);

    let roster = players
        .iter()
        .map(|user_id| Player {
            is_captain: false,
            user_id: *user_id,
            team: None,
            exclude_from_random_captaining: false,
            channel_id_for_picking_session: *pug_thread_channel_id,
            pick_position: None,
        })
        .collect::<Vec<Player>>();

    player_roster_collection.insert_many(roster, None).await?;

    let all_joins_for_game_mode = doc! {
        "game_mode_label": game_mode_label
    };

    game_mode_join_collection
        .delete_many(all_joins_for_game_mode, None)
        .await?;

    let picking_session = PickingSession {
        created: Utc::now(),
        game_mode: game_mode_label.to_string(),
        thread_channel_id: *pug_thread_channel_id,
        pick_sequence,
        last_reset: None,
    };

    picking_session_collection
        .insert_one(picking_session, None)
        .await
}

/// Creates a completed pug record and
/// clears the queue for the game mode
pub async fn register_completed_pug(
    db: Database,
    pug: PugContainer,
) -> Result<InsertOneResult, Error> {
    // FIXME: use sessions
    let collection = db.collection(COMPLETED_PUGS);

    let completed_pug = match pug {
        PugContainer::PickingSession(picking_session) => {
            // gather Player documents linked to the picking sessions's thread/channel

            // Use Player "pick positions" to form blue team and red team arrays for CompletedPug
            // FIXME: implement ^
            let mut blue_team: Vec<u64> = Vec::default();
            let mut red_team: Vec<u64> = Vec::default();

            CompletedPug {
                created: Utc::now(),
                game_mode: picking_session.game_mode,
                thread_channel_id: picking_session.thread_channel_id,
                blue_team,
                red_team,
            }
        }
        PugContainer::CompletedPug(c) => c,
    };

    collection.insert_one(completed_pug, None).await
}

pub async fn pick_player_for_team() -> Result<(), ()> {
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

pub async fn set_pug_channel(
    db: Database,
    channel_id: u64,
    channel_name: Option<String>,
) -> Result<UpdateResult, Error> {
    let collection = db.collection(PUG_CHANNELS);

    let desired_pug_channel = PugChannel {
        channel_id,
        name: channel_name,
    };

    // since we currently only permit one pug channel at a time
    let any = doc! {};

    collection.replace_one(any, desired_pug_channel, None).await
}

pub async fn register_guild_command(
    db: Database,
    guild_command: &ApplicationCommand,
) -> Result<InsertOneResult, Error> {
    db.collection(COMMANDS)
        .insert_one(
            GuildCommand {
                command_id: guild_command.id.0,
                name: guild_command.name.clone(),
            },
            None,
        )
        .await
}

pub async fn clear_guild_commands(db: Database) -> Result<DeleteResult, Error> {
    let all = doc! {};
    db.collection::<GuildCommand>(COMMANDS)
        .delete_many(all, None)
        .await
}

pub async fn save_guild_commands(
    db: Database,
    commands: Vec<ApplicationCommand>,
) -> Result<InsertManyResult, Error> {
    let commands_to_save: Vec<GuildCommand> = commands
        .iter()
        .map(|c| GuildCommand {
            command_id: c.id.0,
            name: c.name.clone(),
        })
        .collect();

    db.collection::<GuildCommand>(COMMANDS)
        .insert_many(commands_to_save, None)
        .await
}

/// Updates a [`Player`] record to grant it captaincy.
pub async fn set_captain(
    db: Database,
    &thread_channel_id: &u64,
    &user_id: &u64,
    team: Team,
) -> Result<Option<Player>, Error> {
    let collection = db.collection(PLAYER_ROSTER);
    let filter = doc! {
        "channel_id_for_picking_session": thread_channel_id as i64,
        "user_id": user_id as i64,
    };

    let update = doc! {
        "team": team,
        "is_captain": true,
        // TODO: why does setting pick_position to None not work
        "pick_position": Bson::Null
    };

    let options = FindOneAndUpdateOptions::builder()
        .upsert(Some(false))
        .build();

    collection
        .find_one_and_update(filter, update, options)
        .await
}
