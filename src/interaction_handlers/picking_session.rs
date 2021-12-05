use anyhow::{bail, Context as AnyhowContext};

use serenity::model::channel::{Channel, ChannelType};
use serenity::model::id::CommandId;
use serenity::utils::MessageBuilder;
use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
};

use crate::command_builder::build_pick;
use crate::db::model::{GuildCommand, PickingSession, Player};
use crate::db::read::{get_current_picking_session, get_picking_session_members};
use crate::utils::captain::{captain_helper, PostSetCaptainAction};
use crate::utils::transform;
use crate::{db, DbClientRef};

// These handlers use the interaction's source channel id to validate whether it is a pug channel/thread,
// then checks/validates the user (e.g. is part of that pug) before going into effect

/// Command handler for /captain.
pub async fn captain(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    // =====================================================================
    // copied
    // =====================================================================
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
    // =====================================================================

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
    let p_s = match get_current_picking_session(db.clone())
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
            picking_session
        }
        None => {
            // ideally, the random captain slash command should've been
            // removed along with the last picking session that completed,
            // so this case never happens
            return Ok("No filled pug available".to_string());
        }
    };

    let response = match captain_helper(&ctx, &guild_id, None, &p_s.thread_channel_id).await {
        Ok(result) => match result {
            PostSetCaptainAction::NeedBlueCaptain => {
                "You are now the cpatain of the red team. Blue team captain needed."
            }
            PostSetCaptainAction::NeedRedCaptain => {
                "You are now the cpatain of the blue team. Red team captain needed."
            }
            PostSetCaptainAction::StartPickingBlue => "Blue captain picks first.",
            PostSetCaptainAction::StartPickingRed => "Red captain picks first.",
        },
        Err(err) => {
            bail!("Failed to perform random captain assignment(s): {}", err);
        }
    };

    Ok(response.to_string())
}

// This command updates `/pick` command options.
pub async fn pick(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    // =====================================================================
    // copied
    // =====================================================================
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };
    let db = client.database(&guild_id.to_string());

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

    // ===== modified below=========
    // ensure this command is being used in the right thread
    let maybe_current_picking_session: Option<PickingSession> =
        get_current_picking_session(db.clone())
            .await
            .context("Tried to fetch current picking session (if any)")?;
    if maybe_current_picking_session.is_none() {
        // ideally, the random captain slash command should've been
        // removed along with the last picking session that completed,
        // so this case never happens
        return Ok("No filled pug available".to_string());
    }
    let picking_session = maybe_current_picking_session.unwrap();
    let is_pug_thread = picking_session.thread_channel_id == guild_channel.id.0;
    if !is_pug_thread {
        let mut response = MessageBuilder::default();
        response
            .push_line("This command cannot be used in this thread.")
            .push("Perhaps you are looking for ")
            .mention(&guild_channel);
        return Ok(response.build());
    }

    // =====================================================================

    let participants: Vec<Player> = get_picking_session_members(db.clone(), &guild_channel.id.0)
        .await
        .context("Tried to fetch a list of `Player`s")?;

    // check that user is a captain
    let current_user_as_captain = participants
        .iter()
        .find(|p| p.is_captain && p.user_id == interaction.user.id.0);
    if current_user_as_captain.is_none() {
        return Ok("You cannot use this command because you are not a captain.".to_string());
    }

    // check that it is this captain's turn to pick
    let participant_count = participants.len();
    let mut teamless_participants = participants
        .clone()
        .into_iter()
        .filter(|p| p.team.is_none())
        .collect::<Vec<Player>>();

    // To determine which team captain should be picking
    let pick_turn = participant_count - teamless_participants.len();

    let team = picking_session
        .pick_sequence
        .get(pick_turn - 1)
        .expect("Picking is not being correctly tracked");

    let user_id_for_user_to_pick = &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("player"))
        .context("The `player` option is missing")?
        .value
        .as_ref()
        .context("The `player` option does not have a value")?
        .as_u64()
        .context(
            "The value of the `player` option (should be a user id) could not be parsed as u64",
        )?;

    // The position of the player pick on their team
    let picking_position = participants
        .iter()
        .filter(|p| p.team.unwrap() == *team)
        .count()
        + 1;

    db::write::pick_player_for_team(
        db.clone(),
        &picking_session.thread_channel_id,
        &user_id_for_user_to_pick,
        team,
        &picking_position,
    )
    .await
    .context("Failed to save the changes of a player pick action.")?;

    // remove player who was just picked from list
    let index = teamless_participants
        .iter()
        .position(|p| p.user_id.eq(user_id_for_user_to_pick))
        .unwrap();
    teamless_participants.remove(index);

    let saved_pick_command: GuildCommand = db::read::find_command(db.clone(), "pick")
        .await
        .context("Failed to search database for a saved pick command")?
        .context(
            "No matching command was found in database when querying for one named \"pick\"",
        )?;

    let remaining_players = transform::players_to_users(&ctx, teamless_participants)
        .await
        .context("Failed to convert pick list `Player`s to `User`s")?;

    let updated_pick_command = build_pick(&remaining_players);
    guild_id
        .edit_application_command(&ctx.http, CommandId(saved_pick_command.command_id), |c| {
            *c = updated_pick_command;
            c
        })
        .await
        .context("Failed to submit updated pick command")?;

    Ok("Okay".to_string())
}

pub async fn reset(ctx: &Context, interaction: &ApplicationCommandInteraction) {
    // validate this channel is a GuildChannel
    // with kind PublicThread
}
