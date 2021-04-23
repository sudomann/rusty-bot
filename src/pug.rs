use chrono::prelude::*;
use linked_hash_set::LinkedHashSet;
use rand::{self, Rng};
use serenity::model::id::UserId;
use std::{
    borrow::Borrow,
    convert::TryInto,
    fmt,
    hash::{Hash, Hasher},
};

#[derive(Eq, PartialEq, Debug, Clone)]
enum PickTurn {
    Blue,
    Red,
}

#[derive(Eq, Debug, Clone)]
pub struct GameMode {
    key: String,
    pub label: String,
    pub player_count: u8,         // must be even
    pick_sequence: Vec<PickTurn>, // because pick sequence only makes sense with even numbers
}

impl Hash for GameMode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}
/*
TODO: evaluate whether derived Clone is good enough
impl Clone for GameMode {
    fn clone(&self) -> Self {
        Self {
            key: self.key.to_owned(),
            label: self.label.to_owned(),
            player_count: self.player_count,
        }
    }
}
*/
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

impl Borrow<String> for GameMode {
    fn borrow(&self) -> &String {
        &self.key
    }
}

impl GameMode {
    pub fn new(label: String, player_count: u8) -> Self {
        let options = [PickTurn::Blue, PickTurn::Red];
        let random_first_pick = &options[rand::thread_rng().gen_range(0, 2)];

        let mut pick_sequence: Vec<PickTurn>;
        match random_first_pick {
            PickTurn::Blue => {
                pick_sequence = vec![PickTurn::Blue];
            }
            PickTurn::Red => {
                pick_sequence = vec![PickTurn::Red];
            }
        }
        for _ in (1..player_count).step_by(2) {
            // Captains alternate double picks when its not first/last pick round
            // The loop the turns for all the
            // picking rounds inbetween the first and last pick
            // Note: It won't run at all for game modes with capacity of 2
            if let Some(turn) = pick_sequence.last() {
                match turn {
                    PickTurn::Blue => {
                        pick_sequence.push(PickTurn::Red);
                        pick_sequence.push(PickTurn::Red);
                    }
                    PickTurn::Red => {
                        pick_sequence.push(PickTurn::Blue);
                        pick_sequence.push(PickTurn::Blue);
                    }
                }
            }
        }
        // the last pick will be the alternative of the first
        // i.e. if red was first pick, blue will be last, and vice versa
        match random_first_pick {
            PickTurn::Blue => {
                pick_sequence = vec![PickTurn::Red];
            }
            PickTurn::Red => {
                pick_sequence = vec![PickTurn::Blue];
            }
        }
        GameMode {
            key: label.to_lowercase(),
            label,
            player_count,
            pick_sequence,
        }
    }

    pub fn key(&self) -> &String {
        &self.key
    }

    pub fn capacity(&self) -> u8 {
        self.player_count
    }
}

#[derive(Eq, Debug, Clone)]
pub struct Player {
    // TODO: `join_datetime` field might interfer with comparison
    // consider manually implementing comparison of UserId's
    user_id: UserId,
    join_datetime: DateTime<Utc>,
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
    }
}

impl PartialEq<UserId> for Player {
    fn eq(&self, other: &UserId) -> bool {
        self.user_id == *other
        // how is this different from
        // &self.user_id == other
    }
}

impl Hash for Player {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.user_id.hash(state);
    }
}

impl Borrow<UserId> for Player {
    /// Facilitates identifying instances of [`PugParticipant`]
    /// within collections, so get, insertion, removal, can be done
    /// by providing a [`UserId`] (borrowed) as argument
    fn borrow(&self) -> &UserId {
        &self.user_id
    }
}

impl Player {
    pub fn new(user_id: UserId) -> Self {
        Player {
            user_id,
            join_datetime: Utc::now(),
        }
    }

    pub fn get_user_id(&self) -> &UserId {
        &self.user_id
    }
}

pub type Participants = LinkedHashSet<Player>;

enum TeamPickAction {
    /// Captain for blue team picked
    BlueCaptain,
    /// Player picked for blue team
    BluePlayer(u8),
    /// Captain for red team picked
    RedCaptain,
    /// One player picked for red team
    RedPlayer(u8),
}

type PickHistory = Vec<TeamPickAction>;
// TODO: where consuming history, use .pop(), .push()

pub struct PickingSession {
    game_mode: GameMode,
    pick_history: PickHistory,
    players: Vec<(u8, UserId)>,
    red_team: LinkedHashSet<(u8, UserId)>,
    blue_team: LinkedHashSet<(u8, UserId)>,
}

impl PickingSession {
    pub fn new(game_mode: &GameMode, players: LinkedHashSet<Player>) -> Self {
        // TODO - start auto captain timer
        let mut enumerated_players: Vec<(u8, UserId)> = Vec::new();
        for (index, player) in players.iter().enumerate() {
            // cast index from usize to u8. We use try_into().unwrap() so it never fails silently
            let player_number = TryInto::<u8>::try_into(index).unwrap() + 1;
            enumerated_players.push((player_number, player.user_id));
        }

        PickingSession {
            game_mode: game_mode.clone(),
            pick_history: Vec::default(),
            players: enumerated_players,
            red_team: LinkedHashSet::default(),
            blue_team: LinkedHashSet::default(),
        }
    }

    pub fn get_red_team(&self) -> &LinkedHashSet<(u8, UserId)> {
        &self.red_team
    }

    pub fn get_blue_team(&self) -> &LinkedHashSet<(u8, UserId)> {
        &self.blue_team
    }

    /// First call which returns [`Ok`] sets captain for one team
    /// and the second call sets captain for the other team.
    ///
    /// The team to which the user gets assigned captain is randomized.
    ///
    /// Subsequent calls return [`Err`] because captains have already been assigned for both teams.
    ///
    /// The [`Err`] contains a tuple which has the form:
    ///
    /// (blue_captain: [`UserId`], red_captain: [`UserId`])
    pub fn set_captain(&mut self, user_id: UserId) -> Result<(), PickError> {
        // first check if captains are already picked
        if let (Some(blue_captain), Some(red_captain)) =
            (self.blue_team.front(), self.red_team.front())
        {
            return Err(PickError::CaptainsExist(
                "Captains have already been selected".to_string(),
                blue_captain.1,
                red_captain.1,
            ));
        }
        let player = self
            .players
            .iter()
            .find(|player| player.1 == user_id)
            .ok_or(PickError::ForeignUser(
                "User trying to become captain is not a player in this pug".to_string(),
            ))?;
        let player_number = player.0;
        self.pick(player_number)
    }

    /// Determines which team to assign the provided user number
    /// then moves them and updates pick history.
    pub fn pick(&mut self, picked_player_number: u8) -> Result<(), PickError> {
        let found_index = self
            .players
            .iter()
            .position(|p| p.0 == picked_player_number)
            .ok_or(PickError::InvalidPlayerNumber(format!(
                "{} is not a valid pick",
                picked_player_number
            )))?;

        let pick_turn = self.pick_history.len();

        let picking_team = self.game_mode.pick_sequence.get(pick_turn).ok_or(
            PickError::PickSequenceInvariantViolation("Pick sequence is empty".to_string()),
        )?;

        // if there have been less than 2 picks, pick history insertions should be the captain variant
        if pick_turn < 2 {
            match picking_team {
                PickTurn::Blue => {
                    self.blue_team.insert(self.players.remove(found_index));
                    self.pick_history.push(TeamPickAction::BlueCaptain);
                }
                PickTurn::Red => {
                    self.red_team.insert(self.players.remove(found_index));
                    self.pick_history.push(TeamPickAction::RedCaptain);
                }
            }
        } else {
            match picking_team {
                PickTurn::Blue => {
                    self.blue_team.insert(self.players.remove(found_index));
                    self.pick_history
                        .push(TeamPickAction::BluePlayer(picked_player_number));
                }
                PickTurn::Red => {
                    self.red_team.insert(self.players.remove(found_index));
                    self.pick_history
                        .push(TeamPickAction::RedPlayer(picked_player_number));
                }
            }
        }

        // check whether only one player remains - if true, auto assign them
        if self.players.len() == 1 {
            let last_player = self
                .players
                .last()
                .ok_or(PickError::PlayersExhausted(
                    "Tried to auto pick last player, but player list was empty.\n
                    This might happen if pick() recursively calls itself more than once."
                        .to_string(),
                ))?
                .0;
            return self.pick(last_player);
        }
        Ok(())
    }

    /// Returns blue team captain - first player in team collection
    pub fn get_blue_captain(&self) -> Option<&(u8, UserId)> {
        self.blue_team.front()
    }

    /// Returns red team captain - first player in team collection
    pub fn get_red_captain(&self) -> Option<&(u8, UserId)> {
        self.red_team.front()
    }

    /// Restores this [`PickingSession`] by clearing captains and team picks
    pub fn reset(&mut self) -> Result<(), String> {
        // self.red_team.drain()
        // self.blue_team.drain()
        self.pick_history.clear();
        Ok(())
    }
}

pub enum PickError {
    CaptainsExist(String, UserId, UserId),
    PlayersExhausted(String),
    HistoryInvariantViolation(String),
    PickSequenceInvariantViolation(String),
    InvalidPlayerNumber(String),
    ForeignUser(String),
}

/*
pub enum OkJoinResult {
    RedTurn,
    BlueTurn,
    PickingComplete(PickingSession),
}
*/
