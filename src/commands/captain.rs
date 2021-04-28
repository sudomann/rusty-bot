use crate::pug::picking_session::{SetCaptainError, SetCaptainSuccess};
use crate::FilledPug;
use itertools::join;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

#[command]
#[aliases("c", "capt", "cap", "iamyourleader")]
async fn captain(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let lock_for_filled_pugs = {
        let data_write = ctx.data.read().await;
        data_write
            .get::<FilledPug>()
            .expect("Expected PugsWaitingToFill in TypeMap")
            .clone()
    };

    let mut filled_pugs = lock_for_filled_pugs.write().await;

    // TODO: review all this use of unwrap(), and try as much as possible to remove
    // and actually handle the None case with perhaps a generic error enum
    let guild_id = msg.guild_id.unwrap();

    let filled_pugs_in_guild = filled_pugs.get_mut(&guild_id);

    if filled_pugs_in_guild.is_none() {
        msg.channel_id
            .say(&ctx.http, "This server does not have data for filled pugs")
            .await?;
        return Ok(());
    }

    let latest_picking_session = filled_pugs_in_guild.unwrap().front_mut();
    if latest_picking_session.is_none() {
        msg.channel_id
            .say(&ctx.http, "No filled pugs to become captain of")
            .await?;
        return Ok(());
    }

    let picking_session = latest_picking_session.unwrap();

    match picking_session.set_captain(msg.author.id) {
        Ok(success) => match success {
            SetCaptainSuccess::NeedBlueCaptain => {
                let response = MessageBuilder::new()
                    .push_bold(msg.author.name.clone())
                    .push(" is captain for the ")
                    .push_bold_line("Red Team")
                    .push("Blue team needs a captain")
                    .build();
                msg.channel_id.say(&ctx.http, response).await?;
            }
            SetCaptainSuccess::NeedRedCaptain => {
                let response = MessageBuilder::new()
                    .push_bold(msg.author.name.clone())
                    .push(" is captain for the ")
                    .push_bold_line("Blue Team")
                    .push("Red team needs a captain")
                    .build();
                msg.channel_id.say(&ctx.http, response).await?;
            }
            SetCaptainSuccess::TwoPlayerAutoPick {
                blue_captain,
                red_captain,
            } => {
                let message = MessageBuilder::new()
                    .push_line("Teams have been auto-selected:")
                    .push_line(format!("**Blue:** {}", blue_captain.mention()))
                    .push(format!("**Red:** {}", red_captain.mention()))
                    .build();
                msg.channel_id.say(&ctx.http, message).await?;
            }
            SetCaptainSuccess::StartPickingBlue => {
                let mut numbered_remaining_players: Vec<String> = Vec::default();
                for (number, user_id) in picking_session.get_remaining() {
                    let name = user_id.to_user(&ctx.http).await?.name;
                    numbered_remaining_players.push(format!("{}) {}", number, name));
                }
                let blue_captain = picking_session.get_blue_captain().unwrap().1;
                let red_captain = picking_session.get_red_captain().unwrap().1;
                let mut response = MessageBuilder::new();
                response
                    .push_line(join(numbered_remaining_players, " :small_orange_diamond: "))
                    .push_line(format!(
                        "**Blue Team:** {}",
                        blue_captain.to_user(&ctx).await?.name
                    ))
                    .push_line(format!(
                        "**Red Team:** {}",
                        red_captain.to_user(&ctx).await?.name
                    ))
                    .push(format!("{} to pick", blue_captain.mention()));

                msg.channel_id.say(&ctx.http, response).await?;
            }
            SetCaptainSuccess::StartPickingRed => {
                // mirrors same logic as arm above
                // TODO: maybe extract into a function to avoid duplication?
                let mut numbered_remaining_players: Vec<String> = Vec::default();
                for (number, user_id) in picking_session.get_remaining() {
                    let name = user_id.to_user(&ctx.http).await?.name;
                    numbered_remaining_players.push(format!("{}) {}", number, name));
                }
                let blue_captain = picking_session.get_blue_captain().unwrap().1;
                let red_captain = picking_session.get_red_captain().unwrap().1;
                let mut response = MessageBuilder::new();
                response
                    .push_line(join(numbered_remaining_players, " :small_orange_diamond: "))
                    .push_line(format!(
                        "**Blue Team:** {}",
                        blue_captain.to_user(&ctx).await?.name
                    ))
                    .push_line(format!(
                        "**Red Team:** {}",
                        red_captain.to_user(&ctx).await?.name
                    ))
                    .push(format!("{} to pick", red_captain.mention()));

                msg.channel_id.say(&ctx.http, response).await?;
            }
        },
        Err(error) => match error {
            SetCaptainError::IsCaptainAlready(m)
            | SetCaptainError::PickFailure(m)
            | SetCaptainError::ForeignUser(m) => {
                msg.channel_id.say(&ctx.http, m).await?;
            }
            SetCaptainError::CaptainSpotsFilled {
                message,
                blue_captain,
                red_captain,
            } => {
                let blue_captain_user = blue_captain.to_user(ctx).await?;
                let red_captain_user = red_captain.to_user(ctx).await?;
                let response = MessageBuilder::new()
                    .push_line(message)
                    .push("Blue: ")
                    .push_bold_line(blue_captain_user.name)
                    .push("Red: ")
                    .push_bold(red_captain_user.name)
                    .build();
                msg.channel_id.say(&ctx.http, response).await?;
            }
        },
    }

    Ok(())
}

#[command]
#[aliases("frc")]
async fn force_random_captains(_ctx: &Context, _msg: &Message, mut _args: Args) -> CommandResult {
    // TODO: get player list, grab 2 random, and call .captain(with_user_id)

    Ok(())
}
