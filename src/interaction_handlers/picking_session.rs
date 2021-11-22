use anyhow::Context as AnyhowContext;

use serenity::model::channel::{Channel, ChannelType};
use serenity::utils::MessageBuilder;
use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
};

use crate::db::read::get_current_picking_session;
use crate::error::SetCaptainOk;
use crate::utils::captain::captain_helper;
use crate::DbClientRef;

// These handlers use the interaction's source channel id to validate whether it is a pug channel/thread,
// then checks/validates the user (e.g. is part of that pug) before going into effect

/// Comand handler for /captain.
pub async fn captain(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    // get all players

    // validate player is part of pug'

    // validate captain position is available

    // give captaincy

    Ok("".to_string())
}

/// A command handler to fill any available captain spots
pub async fn random_captains(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };
    let db = client.database(&guild_id.to_string());

    // FIXME: return early with a message if author is not a moderator/privilleged

    let guild_channel = match interaction
        .channel_id
        .to_channel(&ctx)
        .await
        .context("Tried to obtain `Channel` from a ChannelId")?
    {
        Channel::Guild(channel) => {
            if let ChannelType::PublicThread = channel.kind {
                channel
            } else {
                return Ok("You cannot use this command here".to_string());
            }
        }
        _ => return Ok("You cannot use this command here".to_string()),
    };

    // ensure this command is being used in the right thread
    match get_current_picking_session(db.clone())
        .await
        .context("Tried to fetch current picking session (if any)")?
    {
        Some(picking_session) => {
            let is_pug_thread = picking_session.thread_channel_id == guild_channel.id.0;
            if !is_pug_thread {
                let mut response = MessageBuilder::default();
                response
                    .push_line("This command cannot be used in this thread.")
                    .push("Perhaps you are looking for ")
                    .mention(&guild_channel);
                return Ok(response.build());
            }
        }
        None => {
            // ideally, the random captain slash command should've been
            // removed along with the last picking session that completed,
            // so this case never happens
            return Ok("No filled pug available".to_string());
        }
    }

    // get all players of the picking session associated with this thread

    let participants = get_picking_session_members(db.clone(), &guild_channel.id.0)
        .await
        .context("Tried to fetch a list of `Player`s")?;

    if participants.len() == 0 {
        // this shouldn't ever be true, but just in case...
        return Ok("No players found for this thread".to_string());
    }

    // let is_author_a_participant = participants
    //     .iter()
    //     .any(|p| p.user_id == interaction.user.id.0);
    // if !is_author_a_participant {
    //     return Ok("You are not in this pug".to_string());
    // }

    // assign captain role depending on whether
    // blue/red team needs one

    // delete /captain /nocapt /autocaptain (from db as well)
    Ok("assigned".to_string())
}

/// Performs the player movements to assign them to a team
///
/// Updates pick options accordingly or as necessary.
///
pub async fn captain_helper(db: Database, user_id: u64, team: Team) -> anyhow::Result<()> {
    Ok(())
}

// This command updates `/pick` command options.
pub async fn pick(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    Ok("".to_string())
}

pub async fn reset(ctx: &Context, interaction: &ApplicationCommandInteraction) {
    // validate this channel is a GuildChannel
    // with kind PublicThread
}
