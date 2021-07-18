// use itertools::Itertools;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

use crate::{
    data_structure::CompletedPug,
    utils::time::{Accuracy, HumanTime, Tense},
};
const MAX_HISTORY: usize = 25;
const MIN_HISTORY: usize = 1;
#[command]
#[aliases("lastt", "lasttt")]
#[max_args(1)]
/// This command shows a player list of past pugs.
///
/// __Examples__
///
/// **.last** will show the most recent pug
///
/// **.lastt** will show the second most recent pug
///
/// **.lasttt** will show the third most recent pug
///
/// TODO: NOT IMPLEMENTED - For older pugs, use this command with a number instead.
///
/// e.g. **.last 14**
///
///
async fn last(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    // if there is an argument, try to parse as a number
    let search_distance = if !args.is_empty() {
        match args.single_quoted::<usize>() {
            Ok(value) => {
                if value > MAX_HISTORY || value < MIN_HISTORY {
                    let _ = msg
                        .reply(
                            &ctx.http,
                            format!(
                                "The number must be in the range {} - {}",
                                MIN_HISTORY, MAX_HISTORY
                            ),
                        )
                        .await;
                    return Ok(());
                }
                value
            }
            // if parse fails, return error msg and exit
            Err(err) => {
                let _ = msg
                    .reply(
                        &ctx.http,
                        format!(
                            "You entered an invalid value where a number was expected - {}",
                            err
                        ),
                    )
                    .await;
                return Ok(());
            }
        }
    } else {
        // figure out how many t's and assign to search_distance
        let mut content = msg.content.split_whitespace();
        // first item/word *should* be the command
        let command_text = match content.next() {
            Some(command) => command,
            None => {
                let _ = msg
                    .reply(&ctx.http, "Sorry, failed to parse your input")
                    .await;
                return Ok(());
            }
        };
        // how many "t"s in command
        command_text.to_lowercase().matches("t").count()
    };

    let data_read = ctx.data.read().await;
    let completed_pug_lock = data_read
        .get::<CompletedPug>()
        .expect("Expected CompletedPug in TypeMap");
    let completed_pugs = completed_pug_lock.read().await;
    let completed_pugs_in_guild = completed_pugs.get(&guild_id).unwrap();
    match completed_pugs_in_guild.get(search_distance - 1) {
        Some(picking_session) => {
            let ht = HumanTime::from(picking_session.get_created());
            let mut response = MessageBuilder::new();
            response
                .push_bold(picking_session.get_game_mode().label())
                .push(" ")
                .push_line(format!(
                    "[{}] ago",
                    ht.to_text_en(Accuracy::Precise, Tense::Present)
                ))
                .push_line(format!(
                    "Red Team: {}",
                    picking_session.get_red_team_text(ctx).await?
                ))
                .push(format!(
                    "Blue Team: {}",
                    picking_session.get_blue_team_text(ctx).await?
                ));
            msg.reply(&ctx.http, response).await?;
        }
        None => {
            if search_distance == MIN_HISTORY {
                let _ = msg.reply(&ctx.http, "No completed pugs to show").await;
            } else {
                let _ = msg
                    .reply(
                        &ctx.http,
                        format!("There haven't been up to {} pugs so far", search_distance),
                    )
                    .await;
            }
        }
    }

    Ok(())
}
