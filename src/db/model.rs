use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Guild {
    guild_id: u64,
    enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PugChannel {
    guild_id: u64,
    channel_id: u64,
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameMode {
    guild_id: Guild,
    label: String,
    name: String,
    player_count: u8,
    enlisted_players: Vec<Player>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Player {
    user_id: u64,
    join_datetime: DateTime<Utc>,
}
