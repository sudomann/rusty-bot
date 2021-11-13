
use crate::{db::read::{get_current_picking_session, is_captain_position_available}, interaction_handlers::picking_session::random_captains};
use chrono::{DateTime, Utc};
use mongodb::Database;
use serenity::{client::Context, model::{id::ChannelId, interactions::application_command::ApplicationCommandInteraction}, utils::MessageBuilder};
use tokio::time::{interval, Duration};

const MAX_WAIT_SECS: i64 = 30;


// Intended to be spawned into a new thread, not awaited.
pub async fn countdown(ctx: Context, db: Database, pug_thread_channel_id: ChannelId, interaction: ApplicationCommandInteraction){
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
   
    match random_captains(&ctx, &interaction).await {
        Ok(msg) => {
            let _ = countdown_timeout_alert
            .reply(
                &ctx,
                msg,
            )
            .await;
        },
        // TODO: how to handle this error case
        Err(_e) => {
            let _ = countdown_timeout_alert
            .reply(
                &ctx,
                "Failed to assign random captains. Sorry, try captaining yourselves.",
            )
            .await;
        }
    }
}




