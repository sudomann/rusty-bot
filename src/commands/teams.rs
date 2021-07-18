use crate::data_structure::{CompletedPug, FilledPug};
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    utils::MessageBuilder,
};

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
    let mut response = MessageBuilder::new();
    if maybe_picking_session.is_some() {
        let current_picking_session = maybe_picking_session.unwrap();

        response
            .push_line(
                current_picking_session
                    .get_remaining_player_text(ctx)
                    .await?,
            )
            .push_line("")
            .push_line(format!(
                "**Red Team:** {}",
                current_picking_session.get_red_team_text(ctx).await?
            ))
            .push(format!(
                "**Blue Team:** {}",
                current_picking_session.get_blue_team_text(ctx).await?
            ));
    } else {
        let completed_pugs = completed_pug_lock.read().await;
        let completed_pugs_in_guild = completed_pugs.get(&guild_id).unwrap();
        let maybe_previous_session = completed_pugs_in_guild.last();
        if maybe_previous_session.is_none() {
            msg.reply(&ctx.http, "No pugs to show teams for").await?;
            return Ok(());
        }
        let previous_picking_session = maybe_previous_session.unwrap();
        response
            .push_line(
                previous_picking_session
                    .get_remaining_player_text(ctx)
                    .await?,
            )
            .push_line("")
            .push_line(format!(
                "**Red Team:** {}",
                previous_picking_session.get_red_team_text(ctx).await?
            ))
            .push(format!(
                "**Blue Team:** {}",
                previous_picking_session.get_blue_team_text(ctx).await?
            ));
    };

    msg.reply(&ctx.http, response).await?;

    Ok(())
}
