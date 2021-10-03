use mongodb::error::Error;
use mongodb::results::InsertOneResult;
use mongodb::Database;
use serenity::model::id::GuildId;

use super::model::*;

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
