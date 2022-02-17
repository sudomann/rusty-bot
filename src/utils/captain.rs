use std::array::IntoIter;
use std::collections::HashSet;
use std::iter::FromIterator;

use crate::command_builder::{build_pick, build_reset, build_teams};
use crate::db::model::{GuildCommand, PickingSession, Player, Team};
use crate::db::read::get_picking_session_members;
use crate::db::read::{get_current_picking_session, is_captain_position_available};
use crate::error::SetCaptainErr;
use crate::{db, DbClientRef};
use anyhow::{bail, Context as AnyhowContext};
use chrono::{DateTime, Utc};
use mongodb::Database;
use rand::prelude::{IteratorRandom, SliceRandom};
use serenity::model::id::GuildId;
use serenity::model::interactions::application_command::ApplicationCommand;
use serenity::{client::Context, model::id::ChannelId, utils::MessageBuilder};
use tokio::time::{interval, Duration};
use tracing::info;

const MAX_WAIT_SECS: i64 = 30;

// Intended to be spawned into a new thread, not awaited.
pub async fn autopick_countdown(
    ctx: Context,
    db: Database,
    pug_thread_channel_id: ChannelId,
    guild_id: GuildId,
) {
    let mut interval = interval(Duration::from_secs(1));
    let mut seconds_elapsed;

    let mut countdown_message = pug_thread_channel_id
        .say(
            &ctx,
            format!("Auto captains in about `{}` seconds", MAX_WAIT_SECS),
        )
        .await
        .expect("Auto captain alert to send successfully");
    let mut last_known_reset: Option<DateTime<Utc>> = None;

    loop {
        interval.tick().await;

        seconds_elapsed = Utc::now()
            .signed_duration_since(countdown_message.timestamp)
            .num_seconds();

        let new_update = format!(
            "Auto captains in about `{}` seconds",
            MAX_WAIT_SECS - seconds_elapsed
        );

        // This can be None if:
        // - Between loop iterations, someone leaves
        //   (the user leaving causes the PickingSession to be destroyed)
        //   OR
        // - Between loop iterations, somehow captains and players
        //   are rapidly picked and and the session is moved to CompletedPugs
        let maybe_picking_session = get_current_picking_session(db.clone())
            .await
            .expect("Expected successful db query for the current picking session (if any)");
        if maybe_picking_session.is_none() {
            let final_update = MessageBuilder::new()
                .push_strike_line(new_update)
                .push_italic("Pug was either cancelled/completed")
                .build();
            let _ = countdown_message
                .edit(&ctx.http, |m| m.content(final_update))
                .await;
            return;
        }

        let current_picking_session = maybe_picking_session.unwrap();

        if current_picking_session.thread_channel_id == pug_thread_channel_id.0.to_string() {
            if last_known_reset.is_none() {
                // assign it the latest reset value and proceed with loop
                last_known_reset = current_picking_session.last_reset;
            } else {
                // Check if the last known reset time does not match the latest reset time
                // This indicates someone called the reset command, so we need to terminate this timer
                // so there aren't multiple autocaptain timers running
                if last_known_reset != current_picking_session.last_reset {
                    let final_update = MessageBuilder::new()
                        .push_strike_line(new_update)
                        .push_italic(
                            "Countdown for autocaptain cancelled because the pug was reset",
                        )
                        .build();
                    let _ = countdown_message
                        .edit(&ctx.http, |m| m.content(final_update))
                        .await;
                    return;
                }
            }

            let x = is_captain_position_available(db.clone(), &pug_thread_channel_id.0)
                .await
                .expect("Failure when checking database for players with the captain assignment");

            // cancel the auto captain timer when there are no longer open captain spots
            if !x {
                let final_update = MessageBuilder::new()
                    .push_strike_line(new_update)
                    .push_italic("Countdown for autocaptain cancelled")
                    .build();
                let _ = countdown_message
                    .edit(&ctx.http, |m| m.content(final_update))
                    .await;
                return;
            }

            // this loop should go for no more than 30 secs
            if seconds_elapsed > MAX_WAIT_SECS {
                break;
            }

            let _res = countdown_message
                .edit(&ctx.http, |m| m.content(new_update))
                .await;
        } else {
            let final_update = MessageBuilder::new()
                .push_strike_line(new_update)
                .push_italic("Some new pug has replaced the one this timer was meant for")
                .build();
            let _ = countdown_message
                .edit(&ctx.http, |m| m.content(final_update))
                .await;
            return;
        }
    }

    let countdown_timeout_alert = countdown_message
        .reply(
            &ctx,
            "Random captain assignment because it's been more than 30 seconds",
        )
        .await
        .expect("Expected message declaring timer expiration to send successfully");

    let response = match captain_helper(&ctx, &guild_id, None, &pug_thread_channel_id.0).await {
        Ok(result) => match result {
            PostSetCaptainAction::NeedBlueCaptain => "",
            PostSetCaptainAction::NeedRedCaptain => "",
            PostSetCaptainAction::StartPicking => "",
        },
        Err(_err) => "Failed to assign random captains. Sorry, try captaining yourselves.",
    };

    let _ = countdown_timeout_alert.reply(&ctx, response).await;
}

/// Represents the action to take after performing a captaining operation.
#[derive(Debug)]
pub enum PostSetCaptainAction {
    /// Captain needed for blue team
    NeedBlueCaptain,
    /// Captain needed for red team
    NeedRedCaptain,
    /// Both captains have been selected
    StartPicking,
}

// Checks:
// - user is part of pug
// - right channel
// - user is not already captain
//
// Updates pick options accordingly or as necessary
/// Will attempt to assign user(s) provided as captain to randomly
/// selected team (blue/red) and return the status of the operation.
///
/// When no user is provided, random captains are assigned to
/// fill captains spots if there are any available.
pub async fn captain_helper(
    ctx: &Context,
    guild_id: &GuildId,
    maybe_user_id: Option<u64>,
    thread_channel_id: &u64,
) -> anyhow::Result<PostSetCaptainAction> {
    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
    };
    let db = client.database(&guild_id.to_string());

    // get all players of the picking session associated with this thread

    let participants: Vec<Player> = get_picking_session_members(db.clone(), &thread_channel_id)
        .await
        .context(format!(
            "Tried to fetch a list of `Player`s who are associated with the thread: {}",
            thread_channel_id,
        ))?;

    if participants.len() == 0 {
        // this shouldn't ever be true, but just in case...
        bail!(SetCaptainErr::NoPlayers);
    }

    let mut available_captain_spots =
        HashSet::<_>::from_iter(IntoIter::new([Team::Blue, Team::Red]));

    for p in &participants {
        if p.is_captain {
            available_captain_spots.remove(&p.team.unwrap());
        }
    }

    if available_captain_spots.len() == 0 {
        bail!(SetCaptainErr::CaptainSpotsFilled);
    }

    let mut potential_captains = participants.iter().filter(|p| p.is_captain == false);
    // FIXME: honor /nocapt and exclude players who opted out of being auto-captained
    // .filter(|(_, user_id)| !excluded_players.contains(user_id))
    // exclude them from iterator output

    let operation_outcome = if available_captain_spots.len() == 1 {
        let player = match maybe_user_id {
            Some(_provided_user_id) => {
                let provided_user_id = _provided_user_id.to_string();
                // check whether user is in pug
                let is_a_participant = participants.iter().any(|p| p.user_id == provided_user_id);
                if !is_a_participant {
                    bail!(SetCaptainErr::ForeignUser);
                }

                // check whether user is captain already
                let is_a_captain = participants
                    .iter()
                    .any(|p| p.user_id == provided_user_id && p.is_captain);
                if is_a_captain {
                    bail!(SetCaptainErr::IsCaptainAlready);
                }
                potential_captains
                    .find(|p| p.user_id == provided_user_id)
                    .unwrap()
            }
            None => potential_captains.choose(&mut rand::thread_rng()).unwrap(),
        };

        let team = *available_captain_spots.iter().next().unwrap();

        let player_user_id = player.user_id.parse::<u64>()?;
        db::write::set_one_captain(db.clone(), &thread_channel_id, &player_user_id, team)
            .await
            .context("Database write operation failed when trying to set user as captain")?;

        PostSetCaptainAction::StartPicking
    } else if available_captain_spots.len() == 2 {
        match maybe_user_id {
            Some(user_id) => {
                // select random team on which to assign user as captain
                let team = *Vec::from_iter(&available_captain_spots)
                    .choose(&mut rand::thread_rng())
                    .unwrap();

                db::write::set_one_captain(db.clone(), &thread_channel_id, &user_id, *team)
                    .await
                    .context(
                        "Database write operation failed when trying to set user as captain",
                    )?;

                match team {
                    Team::Blue => PostSetCaptainAction::NeedRedCaptain,
                    Team::Red => PostSetCaptainAction::NeedBlueCaptain,
                }
            }
            None => {
                let mut two_random_players =
                    potential_captains.choose_multiple(&mut rand::thread_rng(), 2);
                let blue_captain = two_random_players.pop().unwrap();
                let red_captain = two_random_players.pop().unwrap();

                let blue_captain_user_id = blue_captain.user_id.parse::<u64>()?;
                let red_captain_user_id = red_captain.user_id.parse::<u64>()?;
                db::write::set_both_captains(
                    db.clone(),
                    &thread_channel_id,
                    &blue_captain_user_id,
                    &red_captain_user_id,
                )
                .await.context(
                    "Database write operation failed when trying to set both captains using one transaction",
                )?;

                // get picking session's pick sequence to determine which color to announce
                // as picking first
                // let picking_session: PickingSession = db::read::get_current_picking_session(db.clone())
                //     .await
                //     .context("")?
                //     .context("Expected there to be an active picking session related to the current captain operation")?;

                // let first_pick_team = picking_session.pick_sequence.get(0).unwrap();
                // match first_pick_team {
                //     Team::Blue => PostSetCaptainAction::StartPicking,
                //     Team::Red => PostSetCaptainAction::StartPickingRed,
                // }
                PostSetCaptainAction::StartPicking
            }
        }
    } else {
        // this should never happen :(
        bail!(SetCaptainErr::InvalidCount);
    };

    match &operation_outcome {
        PostSetCaptainAction::StartPicking => {
            // TODO: perhaps more specific info in this console message
            info!("Clearing /captain /nocapt and /autocaptain commands since both captains have been assigned");

            // delete /captain /nocapt /autocaptain

            let saved_captain_commands: Vec<GuildCommand> =
                db::read::get_captain_related_guild_commands(db.clone())
                    .await
                    .context("Failed to read saved captain-related guild commands from database")?;

            let current_guild_commands = guild_id
                .get_application_commands(&ctx.http)
                .await
                .context("Failed to retrieve list of guild commands from discord")?;

            let commands_to_remove = current_guild_commands
                .into_iter()
                .filter(|c| {
                    saved_captain_commands
                        .iter()
                        .any(|saved_cmd| saved_cmd.command_id == c.id.0)
                })
                .collect::<Vec<ApplicationCommand>>();

            for cmd in &commands_to_remove {
                guild_id
                    .delete_application_command(&ctx.http, cmd.id)
                    .await
                    .context(format!("Failed to remove the {} guild command", cmd.name))?;
            }

            db::write::find_and_delete_guild_commands(
                db.clone(),
                commands_to_remove.into_iter().map(|c| c.name),
            )
            .await
            .context("Attempted and failed to delete captain-related commands from database")?;

            // Perform a set of command creation steps. These steps should
            // occur after both teams are assigned captains and it is time to pick players.
            // 1. Create application commands: /pick and /teams
            // 2. Add database records for the commands
            // !FIXME: for the sake of performance, try filtering and reusing existing participant
            // list from above
            let participants: Vec<Player> =
                get_picking_session_members(db.clone(), &thread_channel_id)
                    .await
                    .context("Tried to fetch a list of `Player`s to convert to `User` objects")?;
            let pick_list = participants
                .into_iter()
                .filter(|p| p.is_captain == false && p.team.is_none());
            let pick_list_as_users = super::transform::players_to_users(&ctx, pick_list)
                .await
                .context("Failed to convert pick list `Player`s to `User`s")?;

            let pick_command = guild_id
                .create_application_command(&ctx.http, |c| {
                    *c = build_pick(&pick_list_as_users);
                    c
                })
                .await
                .context("Failed to create pick command for guild")?;
            let teams_command = guild_id
                .create_application_command(&ctx.http, |c| {
                    *c = build_teams();
                    c
                })
                .await
                .context("Failed to create teams command for guild")?;
            db::write::save_guild_commands(
                db.clone(),
                vec![pick_command, teams_command],
            )
            .await
            .context(
                "Failure when writing records for newly created /pick, /reset and /teams commands",
            )?;
        }
        PostSetCaptainAction::NeedBlueCaptain | PostSetCaptainAction::NeedRedCaptain => {
            // just continue on to return - callers should handle these cases completely
        }
    };
    Ok(operation_outcome)
}
