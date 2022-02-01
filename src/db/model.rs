use chrono::{DateTime, Utc};
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};
use serenity::model::interactions::application_command::ApplicationCommand;
use std::convert::From;

// FIXME: type u64 is not natively supported by mongodb
// so change all usage to String:
// `id.to_string()` to convert u64 --> String
// and
// `my_string.parse::<u64>().unwrap()` to get u64 value

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum Team {
    Blue,
    Red,
}

impl From<Team> for Bson {
    fn from(team: Team) -> Self {
        match team {
            Team::Blue => Bson::String("blue".to_string()),
            Team::Red => Bson::String("red".to_string()),
        }
    }
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
    pub player_user_id: String,
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
    pub thread_channel_id: String,
    pub pick_sequence: Vec<Team>,
    /// Timestamp for tracking latest reset if any. This is useful for
    /// the auto captain countdown to also reset if this value changes.
    pub last_reset: Option<DateTime<Utc>>,
}

/// A model that represents a participant/player
/// involved with a picking session.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Player {
    pub thread_channel_id: String,
    pub is_captain: bool,
    pub exclude_from_random_captaining: bool,
    pub user_id: String,
    pub team: Option<Team>,
    /// The position of your being picked to a team.
    ///
    /// e.g. in a 10 player blitz pug, captain of red team happens to be picking second
    /// (after blue captain - note that captain who picks first is randomly determined),
    /// and picks you first. Your pick position is `1`. The last picked player in such a game mode
    /// (typically 5 players per team) would have a pick position of `4`.
    /// When a player is a captain, they do not get assigned a pick position.
    pub pick_position: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TeamVoiceChat {
    pub category_id: String,
    pub blue_channel_id: String,
    pub red_channel_id: String,
    pub is_deleted_from_guild_channel_list: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CompletedPug {
    pub created: DateTime<Utc>,
    pub game_mode: String,
    pub thread_channel_id: String,
    pub blue_team_captain: String,
    pub blue_team: Vec<String>,
    pub red_team_captain: String,
    pub red_team: Vec<String>,
    pub voice_chat: TeamVoiceChat,
}

// EXPERIMENT BELOW:
// see if the $operation syntax can be used
// to perform partial/tiny updates so we aren't
// // always reading and writing the entire struct :/
// Because several actions to update the session will rely on
// multiple db operations across several collections, session usage is crucial
//
// pub struct Session {
//     thread_channel_id
//     game_mode:
//     created: Datetime<Utc>
//     current_pick_position: 0
//     pick_sequence: Vec<Team>
//     players: HashSet<user_id> set of players pulled out of queue
//     !NOTE: for logical compatibility with existing design,
//     team captain should NOT be included in team Vec
//     blue_captain: Option<user_id>
//     blue_team: Vec<user_id>
//     red_captain: Option<user_id>
//     red_team: Vec<user_id>
//     captain_opt_outs: HashSet<user_id>
//     voice_chat: TeamVoiceChat
// }
