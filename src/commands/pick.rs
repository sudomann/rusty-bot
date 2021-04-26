use crate::{
    pug::{PickError, PickSuccess},
    FilledPug,
};
use itertools::Itertools;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};
use std::collections::HashSet;

#[command]
#[aliases("p")]
#[min_args(1)]
#[max_args(2)]
pub(crate) async fn pick(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let lock_for_filled_pugs = {
        let data_write = ctx.data.read().await;
        data_write
            .get::<FilledPug>()
            .expect("Expected PugsWaitingToFill in TypeMap")
            .clone()
    };

    let mut filled_pugs = lock_for_filled_pugs.write().await;

    // TODO: remove all this use of unwrap()
    // and actually handle the None case with perhaps a generic error enum
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
    let red_captain = picking_session.get_red_captain();

    let blue_captain = picking_session.get_blue_captain();

    // validate that both captains exist
    if blue_captain.is_none() || blue_captain.is_none() {
        msg.channel_id.say(&ctx.http, "Captain needed").await?;
        return Ok(());
    }

    // validate that user is a captain
    let current_team_captains: [&(u8, UserId); 2] = [blue_captain.unwrap(), red_captain.unwrap()];
    if !current_team_captains
        .iter()
        .any(|(_, captain_user_id)| *captain_user_id == msg.author.id)
    {
        msg.reply(&ctx.http, "You are not a captain").await?;
    }

    // TODO: check that its this captain's turn rn

    let player_picks = args
        .iter::<u8>()
        .filter_map(|arg| arg.ok())
        .collect::<HashSet<u8>>();
    for number in player_picks {
        match picking_session.pick(number) {
            Ok(success) => {
                let blue_team = picking_session.get_blue_team();
                let red_team = picking_session.get_red_team();
                let blue_team_text = blue_team
                    .iter()
                    .format_with(" :small_orange_diamond: ", |player, f| {
                        f(&format_args!("{}", player.1.mention()))
                    });
                let red_team_text = red_team
                    .iter()
                    .format_with(" :small_orange_diamond: ", |player, f| {
                        f(&format_args!("{}", player.1.mention()))
                    });

                let mut response = MessageBuilder::new();
                let mut teams = MessageBuilder::new();
                teams
                    .push_bold("Blue Team: ")
                    .push_line(blue_team_text)
                    .push_bold("Red Team: ")
                    .push(red_team_text);

                match success {
                    PickSuccess::BlueTurn => {
                        let captain = picking_session.get_blue_captain().unwrap().1;
                        response
                            .push_line(teams)
                            .user(captain)
                            .push("'s turn to pick");
                    }
                    PickSuccess::RedTurn => {
                        let captain = picking_session.get_red_captain().unwrap().1;
                        response
                            .push_line(teams)
                            .user(captain)
                            .push("'s turn to pick");
                    }
                    PickSuccess::Complete => {
                        response.push_line("Teams have been selected:").push(teams);
                    }
                }

                msg.channel_id.say(&ctx.http, response.build()).await?;
            }
            Err(error) => match error {
                PickError::PlayersExhausted(m)
                | PickError::HistoryInvariantViolation(m)
                | PickError::PickSequenceInvariantViolation(m)
                | PickError::InvalidPlayerNumber(m)
                | PickError::ForeignUser(m) => {
                    msg.reply(&ctx.http, m).await?;
                }
            },
        };
    }

    Ok(())
}
