use linked_hash_set::LinkedHashSet;
use rand::{self, Rng};
use serenity::model::id::UserId;
use std::convert::TryInto;

use super::{game_mode::GameMode, player::Player};

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum PickTurn {
    Blue,
    Red,
}

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

pub enum SetCaptainError {
    IsCaptainAlready(String),
    CaptainSpotsFilled {
        message: String,
        blue_captain: UserId,
        red_captain: UserId,
    },
    PickFailure(String),
    ForeignUser(String),
}

pub enum SetCaptainSuccess {
    /// Captain needed for blue team
    NeedBlue,
    /// Captain needed for red team
    NeedRed,
    /// Both teams have been assigned captains
    Complete,
}

pub enum PickError {
    PlayersExhausted(String),
    HistoryInvariantViolation(String),
    PickSequenceInvariantViolation(String),
    InvalidPlayerNumber(String),
    ForeignUser(String),
}

pub enum PickSuccess {
    /// Blue captain's turn to pick
    BlueTurn,
    /// Red captain's turn to pick
    RedTurn,
    /// All players have been picked.
    ///
    /// Teams can be gotten with [`PickingSession::get_blue_team()`] and [`PickingSession::get_red_team()`]
    Complete,
}

pub struct PickingSession {
    game_mode: GameMode,
    pick_sequence: Vec<PickTurn>,
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
            enumerated_players.push((player_number, player.get_user().id));
        }

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

        // loop only operates if game mode is for more than 2 players
        if game_mode.player_count > 2 {
            // since the player count is 1-based, the loop counter is as well
            // 2 is actually the second index and not first
            let mut counter = 2;
            while counter < game_mode.player_count {
                // This loop operates on the indexes between the first and last
                // Captains alternate double picks when its not first/last pick round,
                // so this loop inserts the double pick turns for all the
                // picking rounds inbetween the first and last pick
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
                counter += 2;
            }
        }

        // the last pick will be the alternative of the first
        // i.e. if red was first pick, blue will be last, and vice versa
        match random_first_pick {
            PickTurn::Blue => {
                pick_sequence.push(PickTurn::Red);
            }
            PickTurn::Red => {
                pick_sequence.push(PickTurn::Blue);
            }
        }

        PickingSession {
            game_mode: game_mode.clone(),
            pick_sequence,
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
    pub fn set_captain(&mut self, user_id: UserId) -> Result<SetCaptainSuccess, SetCaptainError> {
        let blue_captain = self.blue_team.front();
        let red_captain = self.red_team.front();

        if blue_captain.is_some() && red_captain.is_some() {
            return Err(SetCaptainError::CaptainSpotsFilled {
                message: "Captains have already been selected".to_string(),
                blue_captain: blue_captain.unwrap().1,
                red_captain: red_captain.unwrap().1,
            });
        }

        if let Some((_, captain_user_id)) = blue_captain {
            if captain_user_id == &user_id {
                return Err(SetCaptainError::IsCaptainAlready(
                    "You are already captain of blue team.".to_string(),
                ));
            }
        }

        if let Some((_, captain_user_id)) = red_captain {
            if captain_user_id == &user_id {
                return Err(SetCaptainError::IsCaptainAlready(
                    "You are already captain of red team.".to_string(),
                ));
            }
        }

        let player = self
            .players
            .iter()
            .find(|player| player.1 == user_id)
            .ok_or(SetCaptainError::ForeignUser(
                "User trying to become captain is not a player in this pug".to_string(),
            ))?;

        let player_number = player.0;
        match self.pick(player_number) {
            Ok(pick_success) => match pick_success {
                PickSuccess::BlueTurn => Ok(SetCaptainSuccess::NeedBlue),
                PickSuccess::RedTurn => Ok(SetCaptainSuccess::NeedRed),
                PickSuccess::Complete => Ok(SetCaptainSuccess::Complete),
            },
            Err(pick_error) => match pick_error {
                PickError::PlayersExhausted(m)
                | PickError::HistoryInvariantViolation(m)
                | PickError::PickSequenceInvariantViolation(m)
                | PickError::InvalidPlayerNumber(m)
                | PickError::ForeignUser(m) => Err(SetCaptainError::PickFailure(m)),
            },
        }
    }

    /// Determines which team to assign the provided user number
    /// then moves them and updates pick history.
    pub fn pick(&mut self, picked_player_number: u8) -> Result<PickSuccess, PickError> {
        let found_index = self
            .players
            .iter()
            .position(|p| p.0 == picked_player_number)
            .ok_or(PickError::InvalidPlayerNumber(format!(
                "{} is not a valid pick",
                picked_player_number
            )))?;

        let history_length_before_insert = self.pick_history.len();

        let picking_team = self
            .pick_sequence
            .get(history_length_before_insert)
            // e.g. since pick history starts out with length == 0,
            // we use this to retrieve the first PickTurn from pick sequence
            .ok_or(PickError::PickSequenceInvariantViolation(format!(
                "Out of bounds access at index {} in pick sequence",
                history_length_before_insert
            )))?;

        if history_length_before_insert > 2 {
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
        } else {
            // When there havent't been more than 2 picks,
            // history insertions should be the captain variant
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

            // The condition for this if block *should* evaluate false
            // within the recursive call, avoiding an endless loop
        }

        // At this point, it's a good idea ensure we call .len()
        // Captain/player insertion from above will have changed history size
        let next_possible_pick_index = self.pick_history.len();

        Ok(match self.pick_sequence.get(next_possible_pick_index) {
            Some(pick_turn) => match pick_turn {
                PickTurn::Blue => PickSuccess::BlueTurn,
                PickTurn::Red => PickSuccess::RedTurn,
            },
            // out of bounds
            None => PickSuccess::Complete,
        })
    }

    /// Returns blue team captain - first player in team collection
    pub fn get_blue_captain(&self) -> Option<&(u8, UserId)> {
        self.blue_team.front()
    }

    /// Returns red team captain - first player in team collection
    pub fn get_red_captain(&self) -> Option<&(u8, UserId)> {
        self.red_team.front()
    }

    pub fn get_game_mode(&self) -> &GameMode {
        &self.game_mode
    }

    /// Restores this [`PickingSession`] by clearing captains and team picks
    pub fn reset(&mut self) -> Result<(), String> {
        // self.red_team.drain()
        // self.blue_team.drain()
        self.pick_history.clear();
        Ok(())
    }
}
