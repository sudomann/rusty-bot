use crate::{
    pug::picking_session::{PickError, PickSuccess},
    utils::player_user_ids_to_users::*,
    CompletedPug, FilledPug,
};
use itertools::Itertools;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

#[command]
#[aliases("p")]
#[min_args(1)]
#[max_args(2)]
pub(crate) async fn pick(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let lock_for_filled_pugs = {
        let data_write = ctx.data.read().await;
        data_write
            .get::<FilledPug>()
            .expect("Expected FilledPug in TypeMap")
            .clone()
    };

    let mut filled_pugs = lock_for_filled_pugs.write().await;

    // TODO: remove/justify all this use of unwrap()
    // and prefer handling the None case with perhaps a generic error enum
    let guild_id = msg.guild_id.unwrap();

    let filled_pugs_in_guild = filled_pugs.get_mut(&guild_id).unwrap();
    let perhaps_picking_session = filled_pugs_in_guild.front_mut();
    if perhaps_picking_session.is_none() {
        msg.channel_id
            .say(&ctx.http, "No filled pugs to pick players from")
            .await?;
        return Ok(());
    }
    let picking_session = perhaps_picking_session.unwrap();
    let mut picking_session_completed = false;
    let mut peekable = args.iter::<u8>().peekable();
    while let Some(number) = peekable.next() {
        let red_captain = picking_session.get_red_captain();
        let blue_captain = picking_session.get_blue_captain();

        // Validate that both captains exist
        if blue_captain.is_none() || blue_captain.is_none() {
            msg.channel_id.say(&ctx.http, "Captain needed").await?;
            return Ok(());
        }

        // Validate that user is a captain
        let current_team_captains: [&(u8, UserId); 2] =
            [blue_captain.unwrap(), red_captain.unwrap()];
        if !current_team_captains
            .iter()
            .any(|(_, captain_user_id)| *captain_user_id == msg.author.id)
        {
            msg.reply(&ctx.http, "You are not a captain").await?;
            return Ok(());
        }

        // Validate that it's this captain's turn to pick
        // We've already validated that both captains exist, so we can safely unwrap
        let currently_picking_captain = picking_session.currently_picking_captain().unwrap();
        if currently_picking_captain != msg.author.id {
            msg.reply(
                &ctx.http,
                format!("Ignored - {} to pick", currently_picking_captain.mention()),
            )
            .await?;
            return Ok(());
        }

        let mut response = MessageBuilder::new();
        if number.is_err() {
            msg.reply(&ctx.http, format!("Invalid pick: {}", number.unwrap_err()))
                .await?;
            return Ok(());
        }
        let pick_result = picking_session.pick(number.unwrap());
        let remaining = player_user_ids_to_users(ctx, picking_session.get_remaining()).await?;
        let unpicked_players = remaining
            .iter()
            .format_with(" :small_orange_diamond: ", |player, f| {
                f(&format_args!("**{})** {}", player.0, player.1.name))
            });
        // TODO: change these mentions to names
        let blue_team = picking_session
            .get_blue_team()
            .iter()
            .format_with(" :small_orange_diamond: ", |player, f| {
                f(&format_args!("{}", player.1.mention()))
            });

        let red_team = picking_session
            .get_red_team()
            .iter()
            .format_with(" :small_orange_diamond: ", |player, f| {
                f(&format_args!("{}", player.1.mention()))
            });

        response
            .push_line(unpicked_players)
            .push_line("")
            .push_bold("Red Team: ")
            .push_line(red_team)
            .push_bold("Blue Team: ")
            .push_line(blue_team);

        if pick_result.is_ok() {
            match pick_result.ok().unwrap() {
                PickSuccess::BlueTurn => {
                    response
                        .user(picking_session.get_blue_captain().unwrap().1)
                        .push(" to pick");
                }
                PickSuccess::RedTurn => {
                    response
                        .user(picking_session.get_red_captain().unwrap().1)
                        .push(" to pick");
                }
                PickSuccess::Complete => {
                    response.push_line("Teams have been selected!");
                }
            }
        } else {
            match pick_result.err().unwrap() {
                PickError::PlayersExhausted(m)
                | PickError::HistoryInvariantViolation(m)
                | PickError::PickSequenceInvariantViolation(m)
                | PickError::InvalidPlayerNumber(m)
                | PickError::ForeignUser(m) => {
                    response
                        .user(picking_session.currently_picking_captain().unwrap())
                        .push(" to pick");
                    msg.reply(&ctx.http, m).await?;
                }
            }
        }
        if peekable.peek().is_none() {
            // Avoiding responding twice by only responding if there isn't another number to parse
            // When there IS another number, the next iteration will print the final state of team
            msg.channel_id.say(&ctx.http, response).await?;
        }
        if picking_session.is_completed() {
            picking_session_completed = true;
            break;
        }
    }
    if picking_session_completed {
        // move it to completed pugs storage
        {
            let data = ctx.data.read().await;
            let completed_pug_lock = data
                .get::<CompletedPug>()
                .expect("Expected CompletedPug in TypeMap");
            let mut completed_pugs = completed_pug_lock.write().await;
            let completed_pugs_in_guild = completed_pugs.get_mut(&guild_id).unwrap();
            completed_pugs_in_guild.push(filled_pugs_in_guild.pop_front().unwrap());
        }
    }
    Ok(())
}
