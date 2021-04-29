use itertools::Itertools;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    utils::MessageBuilder,
};

use crate::{utils::player_user_ids_to_users::player_user_ids_to_users, FilledPug};

#[command]
#[aliases("team")]
async fn teams(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let lock_for_filled_pugs = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<FilledPug>()
            .expect("Expected PugsWaitingToFill in TypeMap")
            .clone()
    };
    let filled_pugs = lock_for_filled_pugs.read().await;

    let filled_pugs_in_guild = filled_pugs.get(&msg.guild_id.unwrap());

    let pugs = filled_pugs_in_guild.unwrap();
    let last = pugs.front();
    if last.is_none() {
        msg.reply(
            &ctx.http,
            "No pugs to show teams for.\n\
      Take note, at the moment this will only display teams while picking has not concluded",
        )
        .await?;
    }

    let mut response = MessageBuilder::new();

    let picking_session = last.unwrap();

    let remaining = player_user_ids_to_users(ctx, picking_session.get_remaining()).await?;
    let unpicked_players = remaining
        .iter()
        .format_with(" :small_orange_diamond: ", |player, f| {
            f(&format_args!("**{})** {}", player.0, player.1.name))
        });

    let blue_team = player_user_ids_to_users(ctx, picking_session.get_blue_team()).await?;
    let blue_team_text = blue_team
        .iter()
        .format_with(" :small_orange_diamond: ", |player, f| {
            f(&format_args!("{}", player.1.name))
        });

    let red_team = player_user_ids_to_users(ctx, picking_session.get_red_team()).await?;
    let red_team_text = red_team
        .iter()
        .format_with(" :small_orange_diamond: ", |player, f| {
            f(&format_args!("{}", player.1.name))
        });

    response
        .push_line(unpicked_players)
        .push_bold("Blue Team: ")
        .push_line(blue_team_text)
        .push_bold("Red Team: ")
        .push_line(red_team_text);
    msg.reply(&ctx.http, response).await?;

    Ok(())
}
