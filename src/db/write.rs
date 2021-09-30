use super::model::*;
use serenity::model::id::GuildId;

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

pub async fn register_guild(_guild_id: GuildId) -> Result<(), ()> {
    // how to handle a not yet set pug channel, and how to handle setting it?
    // should pug commands be disabled while no pug channel set?
    Ok(())
}
