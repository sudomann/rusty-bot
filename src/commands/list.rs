use crate::{data_structure::PugsWaitingToFill, utils::time::HumanTime};
use itertools::Itertools;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

// if `verbose` argument is true, this player output text
// contains names in addition to player counts
async fn list_helper(ctx: &Context, guild_id: &GuildId, verbose: bool) -> String {
    let data = ctx.data.read().await;
    let lock_for_pugs_waiting_to_fill = data
        .get::<PugsWaitingToFill>()
        .expect("Expected PugsWaitingToFill in TypeMap");

    let pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.read().await;

    let pugs_waiting_to_fill_in_guild = pugs_waiting_to_fill.get(guild_id);

    let pugs = pugs_waiting_to_fill_in_guild.unwrap();
    if pugs.is_empty() {
        return "No game modes registered. Contact admins to run `.addmod`".to_string();
    }
    let mut response = MessageBuilder::new();
    if verbose {
        for (game_mode, players) in pugs.iter() {
            let participants =
                players
                    .iter()
                    .format_with(" :small_orange_diamond: ", |player, f| {
                        let ht = HumanTime::from(player.time_elapsed_since_join());
                        f(&format_args!(
                            "{} [{}]",
                            player.get_user_data().name.clone(),
                            ht,
                        ))
                    });
            response.push_line(format!(
                "**{}** [{}/{}]: {}",
                game_mode.label(),
                players.len(),
                game_mode.capacity(),
                participants
            ));
        }
    } else {
        let pug_occupancy_counts =
            pugs.iter()
                .format_with(" :small_blue_diamond: ", |(game_mode, players), f| {
                    f(&format_args!(
                        "**{}** [{}/{}]",
                        game_mode.label(),
                        players.len(),
                        game_mode.capacity()
                    ))
                });
        response.push(pug_occupancy_counts);
    };
    response.build()
}

#[command("ls")]
#[aliases("list", "lst")]

async fn list(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    msg.reply(&ctx.http, list_helper(ctx, &guild_id, false).await)
        .await?;
    Ok(())
}

#[command("lsa")]
async fn list_all(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    msg.reply(&ctx.http, list_helper(ctx, &guild_id, true).await)
        .await?;
    Ok(())
}
