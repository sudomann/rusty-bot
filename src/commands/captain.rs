use crate::{
    pug::picking_session::{SetCaptainError, SetCaptainSuccess},
    utils::player_user_ids_to_users::*,
    FilledPug,
};
use itertools::Itertools;
use rand::seq::SliceRandom;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

#[command]
#[max_args(0)]
#[aliases("c", "capt", "cap", "iamyourleader")]
async fn captain(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
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
                    .push("**Blue team** needs a captain")
                    .build();
                msg.channel_id.say(&ctx.http, response).await?;
            }
            SetCaptainSuccess::NeedRedCaptain => {
                let response = MessageBuilder::new()
                    .push_bold(msg.author.name.clone())
                    .push(" is captain for the ")
                    .push_bold_line("Blue Team")
                    .push("**Red team** needs a captain")
                    .build();
                msg.channel_id.say(&ctx.http, response).await?;
            }
            SetCaptainSuccess::TwoPlayerAutoPick {
                blue_captain,
                red_captain,
            } => {
                let message = MessageBuilder::new()
                    .push_line("Teams have been auto-selected:")
                    .push(format!("**Red:** {}", red_captain.mention()))
                    .push_line(format!("**Blue:** {}", blue_captain.mention()))
                    .build();
                msg.channel_id.say(&ctx.http, message).await?;
            }
            SetCaptainSuccess::StartPickingBlue | SetCaptainSuccess::StartPickingRed => {
                let remaining =
                    player_user_ids_to_users(ctx, picking_session.get_remaining()).await?;
                let unpicked_players = remaining
                    .iter()
                    .format_with(" :small_orange_diamond: ", |player, f| {
                        f(&format_args!("**{})** {}", player.0, player.1.name))
                    });

                let blue_captain = picking_session.get_blue_captain().unwrap().1;
                let red_captain = picking_session.get_red_captain().unwrap().1;
                let picking_captain = picking_session.currently_picking_captain().unwrap();
                let mut response = MessageBuilder::new();
                response
                    .push_line(unpicked_players)
                    .push_line(format!(
                        "**Blue Team:** {}",
                        blue_captain.to_user(&ctx).await?.name
                    ))
                    .push_line(format!(
                        "**Red Team:** {}",
                        red_captain.to_user(&ctx).await?.name
                    ))
                    .push(format!("{} to pick", picking_captain.mention()));

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
                let mut response = MessageBuilder::new();
                response
                    .push_line(message)
                    .push("Red: ")
                    .push_bold_line(red_captain_user.name)
                    .push("Blue: ")
                    .push_bold(blue_captain_user.name);
                msg.channel_id.say(&ctx.http, response).await?;
            }
        },
    }

    Ok(())
}

#[command]
#[aliases("rc", "frc", "force_random_captain", "force_random_captains")]
#[max_args(0)]
/// Assign captains to random players in filled pug
/// Should work even if one of the captains has already been picked
/// Incomplete
pub async fn random_captains(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let lock_for_filled_pugs = {
        let data_write = ctx.data.read().await;
        data_write
            .get::<FilledPug>()
            .expect("Expected PugsWaitingToFill in TypeMap")
            .clone()
    };

    let mut filled_pugs = lock_for_filled_pugs.write().await;
    let guild_id = msg.guild_id.unwrap();
    let filled_pugs_in_guild = filled_pugs.get_mut(&guild_id).unwrap();

    let perhaps_picking_session = filled_pugs_in_guild.front_mut();
    if perhaps_picking_session.is_none() {
        msg.channel_id
            .say(
                &ctx.http,
                "No filled pugs for which to pick random captains",
            )
            .await?;
        return Ok(());
    }
    let picking_session = perhaps_picking_session.unwrap();

    let mut captains_needed = 0;
    if picking_session.get_blue_captain().is_none() {
        captains_needed += 1;
    }
    if picking_session.get_red_captain().is_none() {
        captains_needed += 1;
    }

    if captains_needed == 0 {
        msg.channel_id
            .say(&ctx.http, "Both teams have captains")
            .await?;
        return Ok(());
    }

    let random_players: Vec<(u8, UserId)> = picking_session
        .get_remaining()
        .choose_multiple(&mut rand::thread_rng(), captains_needed)
        .cloned()
        .collect();

    for (_, user_id) in random_players {
        match picking_session.set_captain(user_id) {
            Ok(success) => {
                match success {
                    SetCaptainSuccess::NeedBlueCaptain | SetCaptainSuccess::NeedRedCaptain => {
                        continue;
                    }
                    SetCaptainSuccess::StartPickingBlue => {
                        msg.channel_id
                            .say(&ctx.http, "Blue Team picks first")
                            .await?;
                    }
                    SetCaptainSuccess::StartPickingRed => {
                        msg.channel_id
                            .say(&ctx.http, "Red Team picks first")
                            .await?;
                    }
                    SetCaptainSuccess::TwoPlayerAutoPick {
                        blue_captain,
                        red_captain,
                    } => {
                        let response = MessageBuilder::new()
                            .push_line("Randomly assigned team(s)/captain(s):")
                            .push_line(format!("**Red:** {}", red_captain.mention()))
                            .push_line(format!("**Blue:** {}", blue_captain.mention()))
                            .build();
                        msg.channel_id.say(&ctx.http, response).await?;
                        if picking_session.is_completed() {
                            // TODO: move completed picking session to a complete pug storage
                            filled_pugs_in_guild.pop_front();
                            break;
                        }
                    }
                };
            }
            Err(err) => match err {
                SetCaptainError::CaptainSpotsFilled {
                    message: m,
                    blue_captain,
                    red_captain,
                } => {
                    // TODO: evaluate whether there's a reasonable situation under which this arm evaluates,
                    // because the if check's above should've handled that already
                    let mut response = MessageBuilder::new();
                    response
                        .push_line(m)
                        .push_line(format!("**Red captain:** {}", red_captain.mention()))
                        .push_line(format!("**Blue captain:** {}", blue_captain.mention()));
                    msg.reply(&ctx.http, response).await?;
                }
                SetCaptainError::IsCaptainAlready(m)
                | SetCaptainError::PickFailure(m)
                | SetCaptainError::ForeignUser(m) => {
                    msg.reply(&ctx.http, m).await?;
                }
            },
        };
    }

    Ok(())
}
