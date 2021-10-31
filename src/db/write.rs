use mongodb::bson::doc;
use mongodb::error::Error;
use mongodb::results::{DeleteResult, InsertManyResult, InsertOneResult, UpdateResult};
use mongodb::Database;
use serenity::model::interactions::application_command::ApplicationCommand;

use super::collection_name::{COMMANDS, GAME_MODES, PUG_CHANNELS};
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
    game_mode: GameMode,
) -> Result<InsertOneResult, Error> {
    let collection = db.collection(GAME_MODES);
    collection.insert_one(game_mode, None).await
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
    commands: Vec<GuildCommand>,
) -> Result<InsertManyResult, Error> {
    db.collection::<GuildCommand>(COMMANDS)
        .insert_many(commands, None)
        .await
}
