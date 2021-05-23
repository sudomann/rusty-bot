use itertools::{join, Itertools};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

use crate::PugsWaitingToFill;

#[command("ls")]
// TODO: add check() which verifies that guild has registered pugs in global data
// TODO: perhaps support game mode arguments to filter output with
// in this case, the filtered gamemodes should be verbose
async fn list(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let lock_for_pugs_waiting_to_fill = {
        let data_write = ctx.data.read().await;
        data_write
            .get::<PugsWaitingToFill>()
            .expect("Expected PugsWaitingToFill in TypeMap")
            .clone()
    };
    let pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.read().await;

    let pugs_waiting_to_fill_in_guild = pugs_waiting_to_fill.get(&msg.guild_id.unwrap());

    let pugs = pugs_waiting_to_fill_in_guild.unwrap();
    let mut response = MessageBuilder::new();

    let pug_member_counts =
        pugs.iter()
            .format_with(" :small_blue_diamond: ", |(game_mode, players), f| {
                f(&format_args!(
                    "**{}** [{}/{}]",
                    game_mode.label(),
                    players.len(),
                    game_mode.capacity()
                ))
            });
    response.push(pug_member_counts).build();
    msg.reply(&ctx.http, response).await?;

    Ok(())
}

#[command("lsa")]
// TODO: show player composition and maybe emphasize the pugs in picking state.
async fn list_all(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let lock_for_pugs_waiting_to_fill = {
        let data_write = ctx.data.read().await;
        data_write
            .get::<PugsWaitingToFill>()
            .expect("Expected PugsWaitingToFill in TypeMap")
            .clone()
    };
    let pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.read().await;

    let pugs_waiting_to_fill_in_guild = pugs_waiting_to_fill.get(&msg.guild_id.unwrap());

    let pugs = pugs_waiting_to_fill_in_guild.unwrap();
    let mut response = MessageBuilder::new();
    for (game_mode, players) in pugs.iter() {
        let player_names = players
            .iter()
            .map(|p| p.get_user().name.clone())
            .collect_vec();
        response.push_line(format!(
            "**{}** *[{}/{}]:* {}",
            game_mode.label(),
            players.len(),
            game_mode.capacity(),
            join(player_names, " :small_orange_diamond: ")
        ));
    }
    msg.reply(&ctx.http, response.build()).await?;

    Ok(())
}
