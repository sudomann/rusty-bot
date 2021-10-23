use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::model::interactions::application_command::ApplicationCommand;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Guild {
    pub guild_id: u64,
    pub disabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PugChannel {
    pub channel_id: u64,
    pub name: Option<String>,
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
pub struct Player {
    pub user_id: u64,
    pub join_datetime: DateTime<Utc>,
}

/// Basically a slimmed down [`serenity::model::interactions::application_command::ApplicationCommand`]
/// with only the field we need to check/store in the database.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct GuildCommand {
    pub command_id: u64,
    pub name: String,
}

impl PartialEq<ApplicationCommand> for GuildCommand {
    fn eq(&self, other: &ApplicationCommand) -> bool {
        other.id == self.command_id
    }
}
