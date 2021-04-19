use chrono::prelude::*;
use linked_hash_set::LinkedHashSet;
use serenity::model::id::UserId;
use std::{
    convert::TryInto,
    fmt,
    hash::{Hash, Hasher},
};

#[derive(Eq, Hash, Debug, Clone)]
pub struct GameMode {
    key: String,
    pub label: String,
    pub player_count: u8, // must be even
}

impl PartialEq<GameMode> for GameMode {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl PartialEq<String> for GameMode {
    fn eq(&self, other: &String) -> bool {
        self.key == other.to_lowercase()
    }
}

impl fmt::Display for GameMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key)
    }
}

impl GameMode {
    pub fn new(label: String, player_count: u8) -> Self {
        GameMode {
            key: label.to_lowercase(),
            label: label,
            player_count: player_count,
        }
    }

    pub fn key(&self) -> &String {
        &self.key
    }

    pub fn capacity(&self) -> u8 {
        self.player_count
    }
}

#[derive(Eq, Debug)]
pub struct PugParticipant {
    // TODO: `join_datetime` field might interfer with comparison
    // consider manually implementing comparison of UserId's
    user_id: UserId,
    join_datetime: DateTime<Utc>,
}

impl PartialEq for PugParticipant {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
    }
}

impl Hash for PugParticipant {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.user_id.hash(state);
    }
}

impl PugParticipant {
    pub fn new(user_id: UserId) -> Self {
        PugParticipant {
            user_id: user_id,
            join_datetime: Utc::now(),
        }
    }
}

#[derive(Debug)]
pub enum Pug {
    Empty,
    // using hashset to guard from duplicates
    Players(LinkedHashSet<PugParticipant>),
}

pub struct PickingSession {
    pick_round: u8,
    players: Vec<(u8, UserId)>,
    red_captain: Option<UserId>,
    red_team: Vec<UserId>,
    blue_captain: Option<UserId>,
    blue_team: Vec<UserId>,
}

impl PickingSession {
    pub fn new(records: LinkedHashSet<PugParticipant>) -> Self {
        // TODO - start auto captain timer
        let mut enumerated_players: Vec<(u8, UserId)> = Vec::new();
        for (index, player) in records.iter().enumerate() {
            // cast index from usize to u8. We use try_into().unwrap() so it never fails silently
            let player_number = TryInto::<u8>::try_into(index).unwrap() + 1;
            enumerated_players.push((player_number, player.user_id));
        }
        PickingSession {
            pick_round: 0,
            players: enumerated_players,
            red_captain: None,
            red_team: Vec::default(),
            blue_captain: None,
            blue_team: Vec::default(),
        }
    }
}
