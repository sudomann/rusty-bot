use itertools::{join, Itertools};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

use crate::{
    data_structure::PugsWaitingToFill,
    utils::parse_game_modes::{parse_game_modes, GameModeError},
};

#[command("ls")]
#[aliases("list", "lst")]
// TODO: add check() which verifies that guild has registered pugs in global data
// TODO: perhaps support game mode arguments to filter output with
// in this case, the filtered gamemodes should be verbose
async fn list(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    if args.is_empty() {
        msg.reply(
            &ctx.http,
            "Ignored\nSpecify the game mode for which you want to list players",
        )
        .await?;
        return Ok(());
    };
    let parsed_game_modes = match parse_game_modes(ctx, &guild_id, args).await {
        Ok(game_modes) => game_modes,
        Err(err) => match err {
            GameModeError::Foreign(m)
            | GameModeError::NoneGiven(m)
            | GameModeError::NoneRegistered(m) => {
                msg.channel_id.say(&ctx.http, m).await?;
                return Ok(());
            }
        },
    };
    let lock_for_pugs_waiting_to_fill = {
        let data_write = ctx.data.read().await;
        data_write
            .get::<PugsWaitingToFill>()
            .expect("Expected PugsWaitingToFill in TypeMap")
            .clone()
    };

    let pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.read().await;
    let pugs_waiting_to_fill_in_guild = pugs_waiting_to_fill.get(&guild_id);

    let pugs = pugs_waiting_to_fill_in_guild.unwrap();
    let mut response = MessageBuilder::new();
    let pug_member_counts = pugs
        .iter()
        // FIXME: implement borrow trait or something on GameMode (i think)
        // so `parsed_game_modes.contains(&game_mode)` in this closure works
        .filter(|(game_mode, _)| parsed_game_modes.contains(game_mode.clone()))
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
async fn list_all(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let lock_for_pugs_waiting_to_fill = data
        .get::<PugsWaitingToFill>()
        .expect("Expected PugsWaitingToFill in TypeMap");

    let pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.read().await;

    let pugs_waiting_to_fill_in_guild = pugs_waiting_to_fill.get(&msg.guild_id.unwrap());

    let pugs = pugs_waiting_to_fill_in_guild.unwrap();
    if pugs.is_empty() {
        msg.reply(
            &ctx.http,
            "No game modes registered. Contact admins to run `.addmod`",
        )
        .await?;
    }
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
