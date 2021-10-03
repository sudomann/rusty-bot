use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Guild {
    pub guild_id: u64,
    pub disabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PugChannel {
    pub guild_id: u64,
    pub channel_id: u64,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameMode {
    pub guild_id: Guild,
    pub label: String,
    pub name: String,
    pub player_count: u8,
    pub enlisted_players: Vec<Player>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Player {
    pub user_id: u64,
    pub join_datetime: DateTime<Utc>,
}
