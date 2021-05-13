use crate::{commands::captain::random_captains, FilledPug};
use chrono::{DateTime, Utc};
use serenity::{
    client::Context,
    framework::standard::{Args, Delimiter},
    model::{channel::Message, id::GuildId},
    utils::MessageBuilder,
};
use tokio::time::{interval, Duration};
use uuid::Uuid;

const MAX_WAIT_SECS: i64 = 30;
/// Be careful when calling this - You must be certain a [`PickingSession`], is in the [`FilledPug`] queue,
/// and particularly the one that you expect this function to operate upon, because it attempts to unwrap().
///
/// Does a 30 second countdown, editing the alert message in the channel to show the countdown value.
/// If the captain slots are taken before countdown completes,
/// `Err()` is returned, containing the countdown value.
/// If the countdown completes, then `Ok(())` is returned
pub async fn do_captain_countdown(ctx: &Context, msg: &Message, guild_id: &GuildId, uuid: &Uuid) {
    let mut interval = interval(Duration::from_secs(1));
    let mut seconds_elapsed;
    let mut countdown_message = msg
        .channel_id
        .say(
            &ctx,
            format!("Auto captains in about `{}` seconds", MAX_WAIT_SECS),
        )
        .await
        .expect("Auto captain alert to send successfully");
    let mut last_known_reset: Option<DateTime<Utc>> = None;
    loop {
        interval.tick().await;
        let lock_for_filled_pugs = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<FilledPug>()
                .expect("Expected FilledPug in TypeMap")
                .clone()
        };

        let filled_pugs = lock_for_filled_pugs.read().await;
        let filled_pugs_in_guild = filled_pugs.get(&guild_id).unwrap();
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
        let maybe_picking_session = filled_pugs_in_guild.front();
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
        if current_picking_session.uuid() == uuid {
            if last_known_reset.is_none() {
                // assign it the latest reset value and proceed with loop
                last_known_reset = current_picking_session.latest_reset();
            } else {
                // Check if the last known reset time does not match the latest reset time
                // This indicates someone called the reset command, so we need to terminate this timer
                // so there aren't multiple autocaptain timers running
                if last_known_reset != current_picking_session.latest_reset() {
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

            let captain_spots_are_filled = current_picking_session.get_blue_captain().is_some()
                && current_picking_session.get_red_captain().is_some();

            if captain_spots_are_filled {
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
            ctx,
            "Random captain assignment because it's been more than 30 seconds",
        )
        .await
        .expect("Expected message declaring timer expiration to send successfully");
    let a = Args::new("", &[Delimiter::Single(' ')]);
    if random_captains(ctx, msg, a).await.is_err() {
        let _ = countdown_timeout_alert
            .reply(
                ctx,
                "Failed to assign random captains. Sorry, try captaining yourselves.",
            )
            .await;
    };

    return;
}
