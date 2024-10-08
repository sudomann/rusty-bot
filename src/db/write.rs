use chrono::Utc;
use mongodb::bson::{doc, Bson};
use mongodb::error::Error;
use mongodb::options::{
    FindOneAndReplaceOptions, FindOneAndUpdateOptions, ReplaceOptions, ReturnDocument,
    UpdateOptions,
};
use mongodb::results::{DeleteResult, InsertManyResult, InsertOneResult, UpdateResult};
use mongodb::{Collection, Database};
use serenity::model::application::Command;

use crate::db::collection_name::PLAYER_ROSTER;

use super::collection_name::{
    COMMANDS, COMPLETED_PUGS, GAME_MODES, GAME_MODE_JOINS, PICKING_SESSIONS, PUG_CHANNELS,
};
use super::model::*;

pub async fn write_new_game_mode(
    db: Database,
    label: String,
    player_count: u64,
) -> Result<InsertOneResult, Error> {
    let collection = db.collection(GAME_MODES);
    let game_mode = GameMode {
        label,
        player_count: player_count as i64,
    };
    collection.insert_one(game_mode, None).await
}

pub async fn delete_game_mode(db: Database, label: String) -> Result<DeleteResult, Error> {
    let collection = db.collection::<GameMode>(GAME_MODES);
    let query = doc! {
        "label": label
    };
    collection.delete_one(query, None).await
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
        "player_user_id": player_user_id.clone() as i64
    };
    let join_record = GameModeJoin {
        game_mode_label: game_mode_label.clone(),
        player_user_id: *player_user_id as i64,
        joined: Utc::now(),
    };
    // create document if no existing
    let options = FindOneAndReplaceOptions::builder()
        .upsert(true)
        .return_document(Some(ReturnDocument::After))
        .build();
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

pub async fn remove_players_from_all_queues(
    db: Database,
    players_user_ids: &Vec<u64>,
) -> Result<DeleteResult, Error> {
    let collection = db.collection::<GameModeJoin>(GAME_MODE_JOINS);

    let participants = players_user_ids
        .iter()
        .map(|id| *id as i64)
        .collect::<Vec<i64>>();

    let filter = doc! {
        "player_user_id": {
            "$in": participants
        }
    };

    collection.delete_many(filter, None).await
}

/// Remove players from the queue of the specified game mode and put them on
/// a roster for a picking session.
///
/// Uses mongodb session feature for atomicity.
pub async fn register_picking_session(
    db: Database,
    pug_thread_channel_id: &u64,
    game_mode_label: &String,
    players: &Vec<u64>,
    pick_sequence: Vec<Team>,
) -> Result<InsertOneResult, Error> {
    // FIXME: use session for atomicity!
    let picking_session_collection = db.collection(PICKING_SESSIONS);
    let player_roster_collection = db.collection(PLAYER_ROSTER);

    let roster = players
        .iter()
        .map(|user_id| Player {
            is_captain: false,
            user_id: *user_id as i64,
            team: None,
            exclude_from_random_captaining: false,
            channel_id_for_picking_session: *pug_thread_channel_id as i64,
            pick_position: None,
        })
        .collect::<Vec<Player>>();

    player_roster_collection.insert_many(roster, None).await?;

    let picking_session = PickingSession {
        created: Utc::now(),
        game_mode: game_mode_label.to_string(),
        thread_channel_id: *pug_thread_channel_id as i64,
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
    completed_pug: &CompletedPug,
) -> Result<InsertOneResult, Error> {
    // FIXME: use sessions
    let completed_pug_collection = db.collection::<CompletedPug>(COMPLETED_PUGS);
    let result = completed_pug_collection
        .insert_one(completed_pug, None)
        .await?;

    let picking_session_query = doc! {
        "thread_channel_id": completed_pug.thread_channel_id.clone()
    };

    let picking_session_collection = db.collection::<PickingSession>(PICKING_SESSIONS);
    picking_session_collection
        .delete_one(picking_session_query, None)
        .await?;

    Ok(result)
}

pub async fn pick_player_for_team(
    db: Database,
    &thread_channel_id: &u64,
    &player_user_id: &u64,
    &team: &Team,
    &pick_position: &usize,
) -> Result<Option<Player>, Error> {
    let collection = db.collection(PLAYER_ROSTER);
    let filter = doc! {
        "channel_id_for_picking_session": thread_channel_id.to_string(),
        "user_id": player_user_id.to_string(),
    };
    let update = doc! {
        "$set": {
            "team": team,
            "pick_position": pick_position.to_string(),
        }
    };
    collection.find_one_and_update(filter, update, None).await
}

pub async fn reset_pug(db: Database, &thread_channel_id: &u64) -> Result<UpdateResult, Error> {
    let collection = db.collection::<Player>(PLAYER_ROSTER);
    let query = doc! {"channel_id_for_picking_session": thread_channel_id as i64};
    let update = doc! {
        "$set": {
            "is_captain": false,
            "exclude_from_random_captaining": false,
            "team": Bson::Null,
            "pick_position": Bson::Null
        }
    };
    collection.update_many(query, update, None).await
}

pub async fn exclude_player_from_random_captaining() -> Result<(), ()> {
    todo!();
}

pub async fn set_pug_channel(
    db: Database,
    channel_id: u64,
    channel_name: Option<String>,
    allowed_game_modes: Vec<String>,
) -> Result<UpdateResult, Error> {
    let collection = db.collection(PUG_CHANNELS);

    let desired_pug_channel = PugChannel {
        channel_id: channel_id as i64,
        name: channel_name,
        allowed_game_modes,
    };

    // since we currently only permit one pug channel at a time
    let any = doc! {};
    let options = ReplaceOptions::builder().upsert(true).build();
    collection
        .replace_one(any, desired_pug_channel, options)
        .await
}

pub async fn register_guild_command(
    db: Database,
    guild_command: &Command,
) -> Result<InsertOneResult, Error> {
    db.collection(COMMANDS)
        .insert_one(
            GuildCommand {
                command_id: guild_command.id.get() as i64,
                name: guild_command.name.clone(),
            },
            None,
        )
        .await
}

/// Delete ALL saved guild commands.
pub async fn clear_guild_commands(db: Database) -> Result<DeleteResult, Error> {
    let all = doc! {};
    db.collection::<GuildCommand>(COMMANDS)
        .delete_many(all, None)
        .await
}

/// Delete any guild commands with names which match any in the provided iterable.
pub async fn find_and_delete_guild_commands<S, I>(
    db: Database,
    command_names: I,
) -> Result<DeleteResult, mongodb::error::Error>
where
    S: AsRef<str>,
    I: IntoIterator<Item = S>,
{
    let collection: Collection<GuildCommand> = db.collection(COMMANDS);
    
    let filter = doc! {
        "name": {
            "$in": command_names.into_iter().map(|s| s.as_ref().to_string()).collect::<Vec<String>>()
        }
    };

    collection.delete_many(filter, None).await
}

pub async fn save_guild_commands(db: Database, commands: Vec<Command>) -> Result<(), Error> {
    for command in &commands {
        let command_to_save = GuildCommand {
            command_id: command.id.get() as i64,
            name: command.name.clone(),
        };

        let filter = doc! { "command_id": command_to_save.command_id };
        let update = doc! { "$set": { "name": command_to_save.name.clone() } };
        let options = UpdateOptions::builder().upsert(true).build();

        db.collection::<GuildCommand>(COMMANDS)
            .update_one(filter, update, options)
            .await?;
    }

    Ok(())
}

/// Updates a [`Player`] record to grant it captaincy.
pub async fn set_one_captain(
    db: Database,
    &thread_channel_id: &u64,
    &user_id: &u64,
    team: Team,
) -> Result<Option<Player>, Error> {
    let collection = db.collection(PLAYER_ROSTER);
    let filter = doc! {
        "channel_id_for_picking_session": thread_channel_id as i64,
        "user_id": user_id as i64
    };

    let update = doc! {
        "$set": {
            "team": team,
            "is_captain": true,
            // TODO: why does setting pick_position to None not work
            "pick_position": Bson::Null
        }
    };

    let options = FindOneAndUpdateOptions::builder()
        .upsert(Some(false))
        .build();

    collection
        .find_one_and_update(filter, update, options)
        .await
}

/// A struct that represents the result of the database operations to:
///
/// Search for two particular [`Player`]s (whom are becoming blue and red team captains)
/// and update their documents to reflect that they are now captains.
pub struct CaptainPair {
    pub blue: Option<Player>,
    pub red: Option<Player>,
}

/// Updates two (for blue and red team) [`Player`] records to grant them captaincy.
pub async fn set_both_captains(
    db: Database,
    &thread_channel_id: &u64,
    &blue_team_captain_user_id: &u64,
    &red_team_captain_user_id: &u64,
) -> Result<CaptainPair, Error> {
    // !FIXME: use sessions
    let collection = db.collection::<Player>(PLAYER_ROSTER);
    let blue_captain_filter = doc! {
        "channel_id_for_picking_session": thread_channel_id as i64,
        "user_id": blue_team_captain_user_id as i64
    };

    let blue_captain_update = doc! {
        "$set": {
            "team": Team::Blue,
            "is_captain": true,
            "pick_position": Bson::Null
        }

    };

    let red_captain_filter = doc! {
        "channel_id_for_picking_session": thread_channel_id as i64,
        "user_id": red_team_captain_user_id as i64,
    };

    let red_captain_update = doc! {
        "$set": {
            "team": Team::Red,
            "is_captain": true,
            "pick_position": Bson::Null
        }

    };

    let options = FindOneAndUpdateOptions::builder()
        .upsert(Some(false))
        .build();

    let blue = collection
        .find_one_and_update(blue_captain_filter, blue_captain_update, options.clone())
        .await?;

    let red = collection
        .find_one_and_update(red_captain_filter, red_captain_update, options)
        .await?;

    Ok(CaptainPair { blue, red })
}

// !FIXME: this is horribly inefficient, but might be fine for relatively
// small quantities of data
pub async fn mark_voice_channels_deleted(
    db: Database,
    channel_ids: Vec<i64>,
) -> Result<UpdateResult, Error> {
    let collection = db.collection::<CompletedPug>(COMPLETED_PUGS);

    let query = doc! {
        "$or": [
            {
                "voice_chat.category.id": {
                    "$in": channel_ids.clone()
                }
            },
            {
                "voice_chat.blue_channel.id": {
                    "$in": channel_ids.clone()
                }
            },
            {
                "voice_chat.red_channel.id": {
                    "$in": channel_ids
                }
            }
        ]
    };

    let update = doc! {
        "voice_chat.category.is_deleted_from_guild_channel_list": true,
        "voice_chat.blue_channel.is_deleted_from_guild_channel_list": true,
        "voice_chat.red_channel.is_deleted_from_guild_channel_list": true
    };

    collection.update_many(query, update, None).await
}
