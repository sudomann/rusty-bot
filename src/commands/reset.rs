use crate::{
    data_structure::{CompletedPug, FilledPug},
    utils::{
        captain_countdown::do_captain_countdown, player_user_ids_to_users::player_user_ids_to_users,
    },
};
use itertools::Itertools;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};
use uuid::Uuid;

#[command]
/// Reset a pug
///
/// Can reset a pug during:
///
/// 1) setting captains, picking players
///
/// 2) after picking all available players
///
/// There are some things to note however:
///
/// - If another pug was queued up, once picking for the current pug concludes,
/// picking automatically commences for the next one in the queue.
///
/// - As a result, when the queue is empty, this command will reset the most recent pug TODO: if no more than 20 mins elapsed
// should be fine to use this commands several times, as it will check that
// one or more captains is needed before doing anything
async fn reset(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let session_uuid: Uuid;
    {
        let (lock_for_filled_pugs, completed_pug_lock) = {
            let data_read = ctx.data.read().await;
            (
                data_read
                    .get::<FilledPug>()
                    .expect("Expected PugsWaitingToFill in TypeMap")
                    .clone(),
                data_read
                    .get::<CompletedPug>()
                    .expect("Expected CompletedPug in TypeMap")
                    .clone(),
            )
        };
        let mut filled_pugs = lock_for_filled_pugs.write().await;

        let filled_pugs_in_guild = filled_pugs.get_mut(&guild_id);

        let pugs = filled_pugs_in_guild.unwrap();
        let maybe_current_picking_session = pugs.front_mut();
        let remaining_ids;
        if maybe_current_picking_session.is_some() {
            let current_picking_session = maybe_current_picking_session.unwrap();
            current_picking_session.reset();
            remaining_ids = current_picking_session.get_remaining().clone();
            session_uuid = current_picking_session.uuid().clone();
        } else {
            // look in completed pug storage
            let mut completed_pugs = completed_pug_lock.write().await;
            let completed_pugs_in_guild = completed_pugs.get_mut(&guild_id).unwrap();
            let maybe_previous_session = completed_pugs_in_guild.pop();
            if maybe_previous_session.is_none() {
                msg.reply(&ctx.http, "No pugs to reset").await?;
                return Ok(());
            }
            let mut previous_picking_session = maybe_previous_session.unwrap();
            previous_picking_session.reset();
            remaining_ids = previous_picking_session.get_remaining().clone();
            session_uuid = previous_picking_session.uuid().clone();
            pugs.push_front(previous_picking_session);
        }
        let remaining_owned = player_user_ids_to_users(ctx, &remaining_ids).await?;
        let unpicked_players = remaining_owned
            .iter()
            .format_with(" :small_orange_diamond: ", |player, f| {
                f(&format_args!("**{})** {}", player.0, player.1.name))
            });
        let mut response = MessageBuilder::new();
        response
            .push_line("Teams were reset")
            .push_line(unpicked_players);
        msg.reply(ctx, response).await?;
    }

    do_captain_countdown(ctx, msg, &guild_id, &session_uuid).await;

    Ok(())
}
