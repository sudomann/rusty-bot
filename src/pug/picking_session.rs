use crate::{
    pug::{game_mode::GameMode, player::Player},
    utils::player_user_ids_to_users::player_user_ids_to_users,
    TeamVoiceChannels,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use linked_hash_set::LinkedHashSet;
use rand::{self, Rng};
use serenity::{client::Context, model::id::UserId};
use std::{collections::HashSet, convert::TryInto, error::Error, mem};
use uuid::Uuid;

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
    NeedBlueCaptain,
    /// Captain needed for red team
    NeedRedCaptain,
    /// Both captains have been selected, and blue team captain picks first
    StartPickingBlue,
    /// Both captains have been selected, and red team captain picks first
    StartPickingRed,
    /// This variant is for two player game modes only, where the "teams"
    /// are comprised of 1 player each, both auto-assigned to
    /// one of either team at random, as captains
    TwoPlayerAutoPick {
        blue_captain: UserId,
        red_captain: UserId,
    },
}

// FIXME: why are some of these variants never constructer? make sure to use or remove
pub enum PickError {
    PlayersExhausted(String),
    #[allow(dead_code)]
    HistoryInvariantViolation(String),
    PickSequenceInvariantViolation(String),
    InvalidPlayerNumber(String),
    #[allow(dead_code)]
    ForeignUser(String),
}

/// Represents the successful assignment of a player to a team, and the variants describe
/// the team that is to pick next, or whether picking is complete/concluded.
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

pub enum SetNoCaptError {
    ForeignUser(String),
    NoCaptainSlotsRemaining(String),
    IsCaptainAlready(String),
    PlayersExhausted(String),
}

pub struct PickingSession {
    // TODO: kurrgan suggestion - potentially implement this as an opt out
    // because the UTPugs guys might not like the randomness - ask them and see
    // uses_captain_randomness = bool
    game_mode: GameMode,
    created: DateTime<Utc>,
    pick_sequence: Vec<PickTurn>,
    pick_history: PickHistory,
    /// A "write once" list of players for conveniently checking pug participants
    player_list: HashSet<UserId>,
    /// A list of number-labelled users who have not yet been assigned to a team
    player_lineup: Vec<(u8, UserId)>,
    auto_captain_exclusions: HashSet<UserId>,
    red_team: LinkedHashSet<(u8, UserId)>,
    blue_team: LinkedHashSet<(u8, UserId)>,
    uuid: Uuid,
    last_reset: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    voice_channels: Option<TeamVoiceChannels>,
}

impl PickingSession {
    pub fn new(
        game_mode: &GameMode,
        players: LinkedHashSet<Player>,
        voice_channels: Option<TeamVoiceChannels>,
    ) -> Self {
        let mut participants: HashSet<UserId> = HashSet::default();
        let mut enumerated_players: Vec<(u8, UserId)> = Vec::default();
        for (index, player) in players.iter().enumerate() {
            // cast index from usize to u8. We use try_into().unwrap() so it never fails silently
            let player_number = TryInto::<u8>::try_into(index).unwrap() + 1;
            enumerated_players.push((player_number, player.get_user_data().id));
            participants.insert(player.get_user_data().id);
        }

        let options = [PickTurn::Blue, PickTurn::Red];
        let random_first_pick = &options[rand::thread_rng().gen_range(0..2)];

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
        let blue_count = pick_sequence
            .iter()
            .filter(|&p| *p == PickTurn::Blue)
            .count();
        let red_count = pick_sequence
            .iter()
            .filter(|&p| *p == PickTurn::Red)
            .count();

        // the variant with lower occurences in the sequence fills the last spot in the vec
        if blue_count < red_count {
            pick_sequence.push(PickTurn::Blue);
        } else {
            pick_sequence.push(PickTurn::Red);
        }

        PickingSession {
            game_mode: game_mode.clone(),
            created: Utc::now(),
            pick_sequence,
            pick_history: Vec::default(),
            player_list: participants,
            player_lineup: enumerated_players,
            red_team: LinkedHashSet::default(),
            blue_team: LinkedHashSet::default(),
            uuid: Uuid::new_v4(),
            last_reset: None,
            voice_channels,
            auto_captain_exclusions: HashSet::default(),
        }
    }

    pub fn get_created(&self) -> DateTime<Utc> {
        self.created.clone()
    }

    pub fn get_red_team(&self) -> &LinkedHashSet<(u8, UserId)> {
        &self.red_team
    }

    pub fn get_blue_team(&self) -> &LinkedHashSet<(u8, UserId)> {
        &self.blue_team
    }

    pub async fn get_blue_team_text(
        &self,
        ctx: &Context,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        Ok(compose_text_for_group(ctx, &self.blue_team, false).await?)
    }
    // TODO; is this proper use of ? operator in conjuction with Ok()??
    // what happens when compose_text_for_group() returns Err??
    pub async fn get_red_team_text(
        &self,
        ctx: &Context,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        Ok(compose_text_for_group(ctx, &self.red_team, false).await?)
    }

    pub async fn get_remaining_player_text(
        &self,
        ctx: &Context,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        Ok(compose_text_for_group(ctx, &self.player_lineup, true).await?)
    }

    #[allow(dead_code)]
    pub fn get_pick_sequence(&self) -> &Vec<PickTurn> {
        &self.pick_sequence
    }

    pub fn get_no_capt_players(&self) -> &HashSet<UserId> {
        &self.auto_captain_exclusions
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
    // TODO: when players manually captain themselves, second to self-captain always gets first pick
    // Maybe as a fix, during second captaining, randomly decide whether to swap captains or not
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
            .player_lineup
            .iter()
            .find(|player| player.1 == user_id)
            .ok_or(SetCaptainError::ForeignUser(
                "User trying to become captain is not a player in this pug".to_string(),
            ))?;

        let player_number = player.0;
        match self.pick(player_number) {
            Ok(pick_success) => match pick_success {
                PickSuccess::BlueTurn => {
                    if self.pick_history.len() == 1
                    // only one item in history (after self.pick() call above)
                    // means only 1 captain assigned
                    {
                        Ok(SetCaptainSuccess::NeedBlueCaptain)
                    } else
                    // more than 1 item in history
                    // means both captains have been assigned,
                    // and we're now picking players
                    {
                        Ok(SetCaptainSuccess::StartPickingBlue)
                    }
                }
                PickSuccess::RedTurn => {
                    // same logic as arm above
                    if self.pick_history.len() == 1 {
                        Ok(SetCaptainSuccess::NeedRedCaptain)
                    } else {
                        Ok(SetCaptainSuccess::StartPickingRed)
                    }
                }
                PickSuccess::Complete => Ok(SetCaptainSuccess::TwoPlayerAutoPick {
                    blue_captain: self.get_blue_captain().unwrap().1,
                    red_captain: self.get_red_captain().unwrap().1,
                }),
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
            .player_lineup
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
                    self.blue_team
                        .insert(self.player_lineup.remove(found_index));
                    self.pick_history
                        .push(TeamPickAction::BluePlayer(picked_player_number));
                }
                PickTurn::Red => {
                    self.red_team.insert(self.player_lineup.remove(found_index));
                    self.pick_history
                        .push(TeamPickAction::RedPlayer(picked_player_number));
                }
            }
        } else {
            // When there havent't been more than 2 picks,
            // history insertions should be the captain variant
            match picking_team {
                PickTurn::Blue => {
                    self.blue_team
                        .insert(self.player_lineup.remove(found_index));
                    self.pick_history.push(TeamPickAction::BlueCaptain);
                }
                PickTurn::Red => {
                    self.red_team.insert(self.player_lineup.remove(found_index));
                    self.pick_history.push(TeamPickAction::RedCaptain);
                }
            }
        }

        // check whether only one player remains - if true, auto assign them

        if self.player_lineup.len() == 1 {
            let last_player = self
                .player_lineup
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

    pub fn get_player_list(&self) -> &HashSet<UserId> {
        &self.player_list
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

    // get list of yet unpicked players
    pub fn get_remaining(&self) -> &Vec<(u8, UserId)> {
        &self.player_lineup
    }

    /// Restores this [`PickingSession`] by clearing captains and team picks
    pub fn reset(&mut self) {
        let players = &mut self.player_lineup;
        /* TODO: Some things are currently being done
        to avoid making closures borrow "too much" and
        forcing you to perform borrow splitting manually, i.e.
        `&mut self.blue_team` and `&mut self.red_team`.
        For now, with nightly you can look into enabling the `capture_disjoint_fields` feature
        */
        players.extend(mem::take(&mut self.blue_team));
        players.extend(mem::take(&mut self.red_team));
        players.sort_by(|a, b| a.0.cmp(&b.0));
        self.auto_captain_exclusions.clear();
        self.pick_history.clear();
        self.last_reset = Some(Utc::now());
    }

    pub fn latest_reset(&self) -> Option<DateTime<Utc>> {
        self.last_reset
    }

    pub fn is_completed(&self) -> bool {
        let full_team_size = self.game_mode.player_count / 2;
        self.player_lineup.len() == 0
            && self.pick_history.len() as u8 == self.game_mode.player_count
            && self.blue_team.len() as u8 == full_team_size
            && self.red_team.len() as u8 == full_team_size
    }

    pub fn currently_picking_captain(&self) -> Option<UserId> {
        let captain = match self.pick_sequence.get(self.pick_history.len()).unwrap() {
            PickTurn::Blue => self.get_blue_captain(),
            PickTurn::Red => self.get_red_captain(),
        };
        if captain.is_none() {
            return None;
        }
        Some(captain.unwrap().1)
    }

    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }

    pub fn exclude_from_autocaptaining(&mut self, user_id: &UserId) -> Result<(), SetNoCaptError> {
        let mut captains_needed = 0;

        // check blue captain
        if let Some((_, blue_captain)) = self.get_blue_captain() {
            if blue_captain == user_id {
                return Err(SetNoCaptError::IsCaptainAlready(
                    "You are already a captain".to_string(),
                ));
            }
        } else {
            captains_needed += 1;
        }

        // check red captain
        if let Some((_, red_captain)) = self.get_red_captain() {
            if red_captain == user_id {
                return Err(SetNoCaptError::IsCaptainAlready(
                    "You are already a captain".to_string(),
                ));
            }
        } else {
            captains_needed += 1;
        }

        if captains_needed == 0 {
            return Err(SetNoCaptError::NoCaptainSlotsRemaining(
                "Both captain spots are taken, so it's time to pick teams".to_string(),
            ));
        }

        // ensure user is a participant in the pug
        if !self
            .player_lineup
            .iter()
            .any(|(_, player_user_id)| player_user_id == user_id)
        {
            return Err(SetNoCaptError::ForeignUser(
                "You are not a partcipant in this pug".to_string(),
            ));
        }

        // don't let too many players .nocapt such that there aren't enough players
        // for the captain timeout process to auto assign
        let number_of_players = self.player_lineup.len();
        let number_of_autocaptain_exclusions = self.auto_captain_exclusions.len();
        let number_of_players_available_for_autocaptain =
            number_of_players - number_of_autocaptain_exclusions;

        if number_of_players_available_for_autocaptain == captains_needed {
            return Err(SetNoCaptError::PlayersExhausted(format!(
                "Ignored. Only {} player(s) remaining to fill the {} available captain spot(s)",
                number_of_players_available_for_autocaptain, captains_needed
            )));
        }

        self.auto_captain_exclusions.insert(*user_id);
        Ok(())
    }
}

async fn compose_text_for_group(
    ctx: &Context,
    player_list: impl IntoIterator<Item = &(u8, UserId)>,
    with_numbers: bool,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let team_text = player_user_ids_to_users(ctx, player_list)
        .await?
        .iter()
        .format_with(" :small_orange_diamond: ", |player, f| {
            if with_numbers {
                f(&format_args!("**{})** {}", player.0, player.1.name))
            } else {
                f(&format_args!("{}", player.1.name))
            }
        })
        .to_string();
    Ok(team_text)
}
