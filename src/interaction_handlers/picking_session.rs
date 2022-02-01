use anyhow::{bail, Context as AnyhowContext};

use serenity::model::channel::{Channel, ChannelType};
use serenity::model::id::{ChannelId, CommandId};
use serenity::utils::MessageBuilder;
use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
};

use crate::command_builder::build_pick;
use crate::db::model::{GuildCommand, PickingSession, Player, Team};
use crate::db::read::{get_current_picking_session, get_picking_session_members};
use crate::error::SetCaptainErr;
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

    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
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
    let picking_session_thread_channel_id = match get_current_picking_session(db.clone())
        .await
        .context("Tried to fetch current picking session (if any)")?
    {
        Some(picking_session) => {
            let thread_channel_id = picking_session.thread_channel_id.parse::<u64>()?;
            let is_pug_thread = thread_channel_id == guild_channel.id.0;
            if !is_pug_thread {
                let mut response = MessageBuilder::default();
                response
                    .push_line("This command cannot be used in this thread.")
                    .push("Perhaps you are looking for ")
                    .mention(&guild_channel);
                return Ok(response.build());
            }
            thread_channel_id
        }
        None => {
            // ideally, the random captain slash command should've been
            // removed along with the last picking session that completed,
            // so this case never happens
            return Ok("No filled pug available".to_string());
        }
    };
    // =====================================================================

    let mut response = MessageBuilder::default();
    match captain_helper(
        &ctx,
        &guild_id,
        Some(interaction.user.id.0),
        &interaction.channel_id.0,
    )
    .await
    {
        Ok(result) => match result {
            PostSetCaptainAction::NeedBlueCaptain => {
                response.push(" is now captain for the red team. Need a captain for blue team.");
            }
            PostSetCaptainAction::NeedRedCaptain => {
                response.push(" is now captain for the blue team. Need a captain for red team.");
            }
            PostSetCaptainAction::StartPicking => {
                response
                    .push("Red Team :red_circle: ")
                    .push_bold("<red_capt> ")
                    .push_line("<red_team>")
                    .push("Blue Team :blue_circle: ")
                    .push_bold("<blue_capt> ")
                    .push_line("<blue_team>")
                    .push_line("")
                    .push("<@blue_capt> :blue_circle: picks first <-- sample");
            }
        },
        Err(err) => {
            if let Some(set_captain_error) = err.downcast_ref::<crate::error::SetCaptainErr>() {
                match set_captain_error {
                    SetCaptainErr::IsCaptainAlready => {
                        response.push("You are already a captain");
                    }
                    SetCaptainErr::CaptainSpotsFilled => {
                        response.push("Both teams have captains already");
                    }
                    SetCaptainErr::ForeignUser => {
                        response.push("You are not in this pug");
                    }
                    SetCaptainErr::MongoError(_)
                    | SetCaptainErr::InvalidCount
                    | SetCaptainErr::Unknown
                    | SetCaptainErr::NoPlayers => {
                        bail!(err);
                    }
                }
            } else {
                bail!(err)
            }
        }
    };
    Ok(response.build())
}

/// A command handler to fill any available captain spots
pub async fn auto_captain(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
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
    let picking_session_thread_channel_id = match get_current_picking_session(db.clone())
        .await
        .context("Tried to fetch current picking session (if any)")?
    {
        Some(picking_session) => {
            let thread_channel_id = picking_session.thread_channel_id.parse::<u64>()?;
            let is_pug_thread = thread_channel_id == guild_channel.id.0;
            if !is_pug_thread {
                let mut response = MessageBuilder::default();
                response
                    .push_line("This command cannot be used in this thread.")
                    .push("Perhaps you are looking for ")
                    .mention(&guild_channel);
                return Ok(response.build());
            }
            thread_channel_id
        }
        None => {
            // ideally, the random captain slash command should've been
            // removed along with the last picking session that completed,
            // so this case never happens
            return Ok("No filled pug available".to_string());
        }
    };

    let response = match captain_helper(&ctx, &guild_id, None, &picking_session_thread_channel_id)
        .await
    {
        Ok(result) => match result {
            PostSetCaptainAction::NeedBlueCaptain | PostSetCaptainAction::NeedRedCaptain => {
                bail!(
                    "Failed to perform random captain assignment(s).\n
                    The helper returned an unacceptable value: `{:?}`.\n
                    `{:?}` is the only valid variant, as there should be no captain spots remaining \
                    after automatic captain selection takes place.",
                    result,
                    PostSetCaptainAction::StartPicking
                );
            }
            PostSetCaptainAction::StartPicking => "!FIXME announce teams and first picking captain",
        },
        Err(err) => {
            if let Some(set_captain_error) = err.downcast_ref::<crate::error::SetCaptainErr>() {
                match set_captain_error {
                    SetCaptainErr::CaptainSpotsFilled => "Both teams have captains already",
                    SetCaptainErr::ForeignUser | SetCaptainErr::IsCaptainAlready => {
                        bail!(
                            "An invalid state `{:?}` was returned by the captain helper function \
                            during auto captaining. The only \"acceptable\" error state after auto captaining is \
                            `{:?}`. Maybe a `Some(user_id)` was passed to the captain helper function by mistake?",
                            set_captain_error, SetCaptainErr::CaptainSpotsFilled
                        );
                    }
                    SetCaptainErr::MongoError(_)
                    | SetCaptainErr::InvalidCount
                    | SetCaptainErr::Unknown
                    | SetCaptainErr::NoPlayers => {
                        bail!(err);
                    }
                }
            } else {
                bail!(err)
            }
        }
    };
    // !FIXME: response here should be to call the helper for the /teams
    // handler, and just send its output

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

    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
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
    let picking_session_thread_channel_id = picking_session.thread_channel_id.parse::<u64>()?;
    let is_pug_thread = picking_session_thread_channel_id == guild_channel.id.0;
    if !is_pug_thread {
        let mut response = MessageBuilder::default();
        response
            .push_line("This command cannot be used in this thread.")
            .push("Perhaps you are looking for ")
            .mention(&ChannelId(picking_session_thread_channel_id));
        return Ok(response.build());
    }

    // =====================================================================

    let participants: Vec<Player> = get_picking_session_members(db.clone(), &guild_channel.id.0)
        .await
        .context("Tried to fetch a list of `Player`s")?;

    // check that user is a captain
    let current_user_as_captain = participants
        .iter()
        .find(|p| p.is_captain && p.user_id == interaction.user.id.0.to_string());
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

    let team_to_assign = picking_session
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
        .filter(|p| p.team.unwrap() == *team_to_assign)
        .count()
        + 1;

    db::write::pick_player_for_team(
        db.clone(),
        &picking_session_thread_channel_id,
        &user_id_for_user_to_pick,
        team_to_assign,
        &picking_position,
    )
    .await
    .context("Failed to save the changes of a player pick action.")?;

    // remove player who was just picked from list
    let index = teamless_participants
        .iter()
        .position(|p| p.user_id == user_id_for_user_to_pick.to_string())
        .unwrap();
    teamless_participants.remove(index);

    // When there's only one player remaining, they get auto assigned
    // to the team lacking a player, and the active picking session
    // is resolved as a completed pug
    if teamless_participants.len() == 1 {
        let last_player = teamless_participants.pop().unwrap();
        let last_player_user_id = last_player.user_id.parse::<u64>()?;

        // Player is assigned to whatever the opposite team is
        let team_with_last_open_spot = match team_to_assign {
            Team::Blue => Team::Red,
            Team::Red => Team::Blue,
        };

        db::write::pick_player_for_team(
            db.clone(),
            &picking_session_thread_channel_id,
            &last_player_user_id,
            &team_with_last_open_spot,
            &picking_position,
        )
        .await
        .context("Failed to auto-assign the last player to a team")?;

        // Use Player "pick positions" to form blue team and red team arrays for CompletedPug
        // FIXME: implement ^
        let mut blue_team = participants
            .clone()
            .into_iter()
            .filter(|p| !p.is_captain && p.team == Some(Team::Blue))
            .map(|p| p.user_id.to_string())
            .collect::<Vec<String>>();
        let mut red_team = participants
            .clone()
            .into_iter()
            .filter(|p| !p.is_captain && p.team == Some(Team::Red))
            .map(|p| p.user_id.to_string())
            .collect::<Vec<String>>();

        // add just picked player and last remaining player to these local,
        // up-to-date team lists
        match team_to_assign {
            Team::Blue => {
                blue_team.push(user_id_for_user_to_pick.to_string());
                red_team.push(last_player.user_id.to_string());
            }
            Team::Red => {
                blue_team.push(last_player.user_id.to_string());
                red_team.push(user_id_for_user_to_pick.to_string());
            }
        }

        let blue_team_captain = participants
            .iter()
            .find(|p| p.is_captain && p.team == Some(Team::Blue))
            .unwrap();
        let red_team_captain = participants
            .iter()
            .find(|p| p.is_captain && p.team == Some(Team::Red))
            .unwrap();

        let completed_pug = transform::resolve_to_completed_pug(
            &ctx,
            db.clone(),
            picking_session,
            guild_channel.position,
            blue_team_captain.user_id.to_string(),
            blue_team,
            red_team_captain.user_id.to_string(),
            red_team,
        )
        .await
        .context("Failed to promote active pug to completed pug status")?;

        // Unwrapping like this is probably fine because it comes from a String
        // (which came from a proper u64) that has not been moved about (e.g. transimitted to/from db)
        // or tampered with.
        let red_team_voice_channel = ChannelId(
            completed_pug
                .voice_chat
                .red_channel_id
                .parse::<u64>()
                .unwrap(),
        );
        let blue_team_voice_channel = ChannelId(
            completed_pug
                .voice_chat
                .blue_channel_id
                .parse::<u64>()
                .unwrap(),
        );

        let response = MessageBuilder::new()
            .push_line(
                "All players have been picked. Click your team color to join the voice channel:",
            )
            .mention(&red_team_voice_channel)
            .push_line(" player1 - player2 - TODO")
            .mention(&blue_team_voice_channel)
            .push_line(" player1 - player2 - TODO")
            .build();
        return Ok(response);
    }

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

pub async fn reset(
    _ctx: &Context,
    _interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    // validate this channel is a GuildChannel
    // with kind PublicThread

    // check for an active pug

    // Clear all captains and picks

    // Restart autocap timer

    Ok("Sorry, this feature is incomplete".to_string())
}
