use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::model::interactions::application_command::ApplicationCommand;

use crate::interaction_handlers::picking_session::Team;

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
    pub label: String,
    pub player_count: u64,
}

/// A model that represents a player who has joined the waiting queue for a certain game mode
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameModeJoin {
    pub game_mode_label: String,
    pub player_user_id: u64,
    pub joined: DateTime<Utc>,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PickingSession {
    pub created: DateTime<Utc>,
    pub game_mode: String,
    /// Channel Id of the thread created for managing/organizing  
    /// a filled pug. This is the primary identifier of a picking session.
    pub thread_channel_id: u64,
    pub pick_sequence: Vec<Team>,
}

/// A model that represents a participant/player
/// during a picking session
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Player {
    pub channel_id_for_picking_session: u64,
    pub is_captain: bool,
    pub exclude_from_random_captaining: bool,
    pub user_id: u64,
    pub team: Option<Team>,
    /// The position of your being picked to a team.
    ///
    /// e.g. in a 10 player blitz pug, captain of red team happens to be picking second
    /// (after blue captain - note that captain who picks first is randomly determined),
    /// and picks you first. Your pick position is `1`. The last picked player in such a game mode
    /// (typically 5 players per team) would have a pick position of `4`.
    /// WHen a player is a captain, they do not get assigned a pick position.
    pub pick_position: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CompletedPug {
    pub created: DateTime<Utc>,
    pub game_mode: String,
    pub thread_channel_id: u64,
    pub blue_team: Vec<u64>,
    pub red_team: Vec<u64>,
}
