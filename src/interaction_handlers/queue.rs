use anyhow::Context as AnyhowContext;
use chrono::Utc;
use rand::seq::SliceRandom;
use serenity::client::Context;
use serenity::model::channel::{Channel, ChannelType};
use serenity::model::id::UserId;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::utils::MessageBuilder;

use crate::command_builder::{build_autocaptain, build_captain, build_nocaptain, build_reset};
use crate::db::model::PickingSession;
use crate::db::read::{find_game_mode, get_game_mode_queue};
use crate::db::write::{
    add_player_to_game_mode_queue, create_picking_session, save_guild_commands,
};
use crate::utils::{captain, transform};
use crate::DbClientRef;

// FIXME: add anyhow context to all ? operator usage
pub async fn join(
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

    let guild_channel = match interaction
        .channel_id
        .to_channel(&ctx)
        .await
        .context("Tried to obtain `Channel` from a ChannelId")?
    {
        Channel::Guild(channel) => {
            if let ChannelType::Text = channel.kind {
                channel
            } else {
                return Ok("You cannot use this command here".to_string());
            }
        }
        _ => return Ok("You cannot use this command here".to_string()),
    };

    let current_user_id = &interaction.user.id.0;

    let arg = &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("game_mode"))
        .context("The `game_mode` option is missing")?
        .value
        .as_ref()
        .context("The `game_mode` option does not have a value")?
        .as_str()
        .context("Somehow, the value of the `game_mode` option is not a string")?;
    let game_mode_arg = arg.to_string();
    let maybe_game_mode = find_game_mode(db.clone(), &game_mode_arg).await?;
    if maybe_game_mode.is_none() {
        return Ok("No game mode found with this name".to_string());
    }
    let game_mode = maybe_game_mode.unwrap();
    // TODO: maybe add some anyhow context for potential error?
    let mut queue = get_game_mode_queue(db.clone(), &game_mode.label).await?;
    let user_is_in_queue = queue
        .iter()
        .any(|join_record| join_record.player_user_id == *current_user_id);

    if user_is_in_queue {
        return Ok("You already joined".to_string());
    }

    let queue_not_yet_filled = queue.len() as u64 + 1 < game_mode.player_count;

    if queue_not_yet_filled {
        // add player to game mode queue and exit
        add_player_to_game_mode_queue(db.clone(), &game_mode.label, &current_user_id)
            .await
            .context(format!(
                "Failed to add user {} to {} game mode",
                &current_user_id, &game_mode.label
            ))?;

        return Ok("You were successfully added to the waiting queue".to_string());
    }

    let mut players = queue
        .iter_mut()
        .map(|j| j.player_user_id)
        .collect::<Vec<u64>>();
    // no need to insert this user into the queue
    // at the database level as it'll soon be cleared
    players.push(current_user_id.clone());

    let mut announcement = MessageBuilder::default();
    announcement
        .push_bold(&game_mode.label)
        .push_line(" filled!");
    for player in players.iter() {
        announcement.mention(&UserId(*player));
    }

    let m = interaction.channel_id.say(&ctx.http, announcement).await?;

    let pug_thread = interaction
        .channel_id
        .create_public_thread(&ctx.http, m, |c| {
            c.name(format!("{} - {}", &game_mode.label, Utc::now()))
                .auto_archive_duration(1440)
                .kind(ChannelType::PublicThread)
        })
        .await?;

    // We need to generate a pick sequence first
    let pick_sequence = crate::utils::pick_sequence::generate(&game_mode.player_count);

    if game_mode.player_count == 2 {
        // two-player game modes do not undergo a picking process,
        // so we simply register a completed pug:

        let autocompleted_picking_session = PickingSession {
            created: Utc::now(),
            game_mode: game_mode.label.clone(),
            thread_channel_id: pug_thread.id.0,
            pick_sequence,
            last_reset: None,
        };

        // players assigned to random team,
        // with empty team lists
        let first_random_player = players.choose(&mut rand::thread_rng()).unwrap();
        let remaining_player = players.last().unwrap();
        let completed_pug = transform::resolve_to_completed_pug(
            &ctx,
            db.clone(),
            autocompleted_picking_session,
            guild_channel.position,
            first_random_player.to_string(),
            vec![],
            remaining_player.to_string(),
            vec![],
        )
        .await
        .context(
            "Failed to auto promote 2 player game mode to completed pug \
            status after bypassing picking session",
        )?;

        // Unwrapping like this is probably fine because it comes from a String
        // (which came from a proper u64) that has not been moved about or tampered with.
        let red_player = UserId(completed_pug.red_team_captain.parse::<u64>().unwrap());
        let blue_player = UserId(completed_pug.blue_team_captain.parse::<u64>().unwrap());

        // then announce auto-picked team colors in pug thread
        let response = MessageBuilder::new()
            .push_line("Randomly assigned team colors:")
            .push("Red :red_circle: ")
            .mention(&red_player)
            .push_line("")
            .push("Blue: :blue_circle: ")
            .mention(&blue_player)
            .build();

        interaction.channel_id.say(&ctx.http, response).await?;
    } else {
        let _working_in_thread = pug_thread.clone().start_typing(&ctx.http);

        // create picking session with these players in it
        create_picking_session(
            db.clone(),
            &pug_thread.id.0,
            &game_mode.label,
            &players,
            pick_sequence,
        )
        .await?;

        // create commands (and register in db): /captain /autocaptain /nocapt /reset

        // TODO: perhaps it's a good idea to manually handle the error case here
        // i.e. attempt to delete any commands created so far?
        let autocaptain_cmd = guild_id
            .create_application_command(&ctx.http, |c| {
                *c = build_autocaptain();
                c
            })
            .await?;
        let captain_cmd = guild_id
            .create_application_command(&ctx.http, |c| {
                *c = build_captain();
                c
            })
            .await?;
        let nocaptain_cmd = guild_id
            .create_application_command(&ctx.http, |c| {
                *c = build_nocaptain();
                c
            })
            .await?;
        let reset_cmd = guild_id
            .create_application_command(&ctx.http, |c| {
                *c = build_reset();
                c
            })
            .await?;

        save_guild_commands(
            db.clone(),
            vec![autocaptain_cmd, captain_cmd, nocaptain_cmd, reset_cmd],
        )
        .await?;

        // TODO: update options on the following commands:
        // /list

        // spawn a timer which will auto pick captains if necessary
        let ctx_clone = ctx.clone();
        tokio::spawn(captain::autopick_countdown(
            ctx_clone,
            db.clone(),
            pug_thread.id,
            interaction.clone(),
        ));
    }

    return Ok(
        "If these participants were in the queue of any other game mode, \
    they have been removed"
            .to_string(),
    );
}

pub async fn leave(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    let _working = interaction.channel_id.start_typing(&ctx.http);
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>().unwrap().clone()
    };
    let _db = client.database(&guild_id.to_string());
    Ok("You were successfully removed from the waiting queue".to_string())
}
