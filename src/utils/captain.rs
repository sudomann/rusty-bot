use crate::db::model::Team;
use crate::db::read::get_picking_session_members;
use crate::db::read::{get_current_picking_session, is_captain_position_available};
use crate::db::write::set_captain;
use anyhow::Context as AnyhowContext;
use chrono::{DateTime, Utc};
use mongodb::Database;
use rand::prelude::{IteratorRandom, SliceRandom};
use serenity::{
    client::Context,
    model::{id::ChannelId, interactions::application_command::ApplicationCommandInteraction},
    utils::MessageBuilder,
};
use tokio::time::{interval, Duration};

const MAX_WAIT_SECS: i64 = 30;

// Intended to be spawned into a new thread, not awaited.
pub async fn autopick_countdown(
    ctx: Context,
    db: Database,
    pug_thread_channel_id: ChannelId,
    interaction: ApplicationCommandInteraction,
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

        if current_picking_session.thread_channel_id == pug_thread_channel_id.0 {
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

    let response = match captain_helper(db.clone(), None, &pug_thread_channel_id.0).await {
        Ok(result) => match result {
            SetCaptainOk::NeedBlueCaptain => "",
            SetCaptainOk::NeedRedCaptain => "",
            SetCaptainOk::StartPickingBlue => "",
            SetCaptainOk::StartPickingRed => "",
        },
        Err(err) => "Failed to assign random captains. Sorry, try captaining yourselves.",
    };

    let _ = countdown_timeout_alert.reply(&ctx, response).await;
}

/// Represents the action to take after performing a captaining operation.
pub enum PostSetCaptainAction {
    /// Captain needed for blue team
    NeedBlueCaptain,
    /// Captain needed for red team
    NeedRedCaptain,
    /// Both captains have been selected, and blue team captain picks first
    StartPickingBlue,
    /// Both captains have been selected, and red team captain picks first
    StartPickingRed,
}

// Checks:
// - user is part of pug
// - right channel
// - user is not already captain
//
// Updates pick options accordingly or as necessary
/// Will attempt to assign user(s) provided as captain to randomly
/// selected team (blue/green) and return the status of the operation.
///
/// When no user is provided, random captains are assigned to
/// fill captains spots if there are any available.
pub async fn captain_helper(
    db: Database,
    maybe_user_id: Option<u64>,
    thread_channel_id: &u64,
) -> Result<SetCaptainOk, SetCaptainErr> {
    // get all players of the picking session associated with this thread

    let participants = get_picking_session_members(db.clone(), &thread_channel_id)
        .await
        .context("Tried to fetch a list of `Player`s")?;

    if participants.len() == 0 {
        // this shouldn't ever be true, but just in case...
        return Ok("No players found for this thread".to_string());
    }

    // assign captain role depending on whether
    // blue/red team needs one

    let existing_captains = participants.iter().filter(|p| p.is_captain);
    let captains_needed = 2 - existing_captains.count();

    if captains_needed == 0 {
        return Err(SetCaptainErr::CaptainSpotsFilled {
            blue_captain: val,
            red_captain: val,
        });
    }

    match maybe_user_id {
        Some(user_id) => {
            // check whether user is in pug
            let is_a_participant = participants.iter().any(|p| p.user_id == user_id);
            if !is_a_participant {
                return Err(SetCaptainErr::ForeignUser);
            }

            // check whether user is captain already
            let is_a_captain = participants
                .iter()
                .any(|p| p.user_id == user_id && p.is_captain);
            if is_a_captain {
                return Err(SetCaptainErr::ForeignUser);
            }

            let team_to_assign = if captains_needed == 2 {
                // assign user as captain of random team
                vec![Team::Blue, Team::Red]
                    .choose(&mut rand::thread_rng())
                    .unwrap()
                    .clone()
            } else if captains_needed == 1 {
                // determine what team color is available for captaining
                // FIXME: find better way for these errors to bubble up, rather than panic via expect()
                let captain = existing_captains
                    .last()
                    .expect("Expected 1 players in vector of existing captains");

                // assign the user to the team color that is not taken by first captain
                match captain
                    .team
                    .expect("Expected a captained player to have a team designation")
                {
                    Team::Blue => Team::Red,
                    Team::Red => Team::Blue,
                }
            } else {
                // FIXME: this should not happen :(
                return Err(SetCaptainErr::InvalidCount);
            };

            set_captain(db.clone(), thread_channel_id, &user_id, team_to_assign).await;
        }
        None => {
            let potential_captains = participants.iter().filter(|p| p.is_captain == false);

            let random_players = potential_captains
                // FIXME: honor /nocapt and exclude players who opted out of being auto-captained
                // .filter(|(_, user_id)| !excluded_players.contains(user_id))
                .choose_multiple(&mut rand::thread_rng(), captains_needed);

            let mut response = MessageBuilder::default();
            for p in random_players {
                set_captain(db.clone(), thread_channel_id, &p.user_id, team).await;
            }
        }
    }

    // !FIXME: as necessary, delete /captain /nocapt /autocaptain (from db as well)

    Ok(())
}
