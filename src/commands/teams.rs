use itertools::Itertools;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    utils::MessageBuilder,
};

use crate::{utils::player_user_ids_to_users::player_user_ids_to_users, CompletedPug, FilledPug};

#[command]
#[aliases("team", "picking", "pickings")]
pub async fn teams(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
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
    let filled_pugs = lock_for_filled_pugs.read().await;

    let filled_pugs_in_guild = filled_pugs.get(&guild_id);

    let pugs = filled_pugs_in_guild.unwrap();
    let maybe_picking_session = pugs.front();
    let remaining_ids;
    let blue_team_ids;
    let red_team_ids;
    if maybe_picking_session.is_some() {
        let current_picking_session = maybe_picking_session.unwrap();
        remaining_ids = current_picking_session.get_remaining().clone();
        blue_team_ids = current_picking_session.get_blue_team().clone();
        red_team_ids = current_picking_session.get_red_team().clone();
    } else {
        let completed_pugs = completed_pug_lock.read().await;
        let completed_pugs_in_guild = completed_pugs.get(&guild_id).unwrap();
        let maybe_previous_session = completed_pugs_in_guild.last();
        if maybe_previous_session.is_none() {
            msg.reply(&ctx.http, "No pugs to show teams for").await?;
            return Ok(());
        }
        let previous_picking_session = maybe_previous_session.unwrap();

        remaining_ids = previous_picking_session.get_remaining().clone();
        blue_team_ids = previous_picking_session.get_blue_team().clone();
        red_team_ids = previous_picking_session.get_red_team().clone();
    }

    let mut response = MessageBuilder::new();

    let remaining_owned = player_user_ids_to_users(ctx, &remaining_ids).await?;
    let unpicked_players = remaining_owned
        .iter()
        .format_with(" :small_orange_diamond: ", |player, f| {
            f(&format_args!("**{})** {}", player.0, player.1.name))
        });

    let blue_team = player_user_ids_to_users(ctx, &blue_team_ids).await?;
    let blue_team_text = blue_team
        .iter()
        .format_with(" :small_orange_diamond: ", |player, f| {
            f(&format_args!("{}", player.1.name))
        });

    let red_team = player_user_ids_to_users(ctx, &red_team_ids).await?;
    let red_team_text = red_team
        .iter()
        .format_with(" :small_orange_diamond: ", |player, f| {
            f(&format_args!("{}", player.1.name))
        });

    response
        .push_line(unpicked_players)
        .push_line("")
        .push_bold("Red Team: ")
        .push_line(red_team_text)
        .push_bold("Blue Team: ")
        .push_line(blue_team_text);
    msg.reply(&ctx.http, response).await?;

    Ok(())
}
