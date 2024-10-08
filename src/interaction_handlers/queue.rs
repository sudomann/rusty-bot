use anyhow::Context as AnyhowContext;
use chrono::Datelike;
use chrono::Utc;
use itertools::Itertools;
use mongodb::Database;
use serenity::all::CommandInteraction;
use serenity::all::CreateThread;
use serenity::client::Context;
use serenity::model::channel::{Channel, ChannelType, GuildChannel};
use serenity::model::id::{GuildId, UserId};
use serenity::utils::MessageBuilder;
use std::collections::HashMap;

use crate::command_builder::{build_autocaptain, build_captain, build_nocaptain, build_reset};
use crate::db::model::{GameMode, GameModeJoin, PickingSession};
use crate::db::read::{find_game_mode, get_game_mode_queue};
use crate::db::write::{
    add_player_to_game_mode_queue, register_picking_session, save_guild_commands,
};
use crate::utils::{captain, transform};
use crate::{db, DbClientRef};

use super::IntendedGameMode;

// FIXME: add anyhow context to all ? operator usage
// !TODO: lots of duplicate code in this whole module

pub async fn join(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<String> {
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
            if let ChannelType::Text = channel.kind {
                channel
            } else {
                return Ok("You cannot use this command here".to_string());
            }
        }
        _ => return Ok("You cannot use this command here".to_string()),
    };

    let current_user_id = &interaction.user.id.get();

    let game_mode_target = match &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("game_mode"))
    {
        Some(game_mode_option) => {
            let arg = game_mode_option
                .value
                .as_str()
                .context("Somehow, the value of the `game_mode` option is not a string")?
                .to_string();
            IntendedGameMode::Single(arg)
        }
        None => IntendedGameMode::All,
    };

    join_helper(
        &ctx,
        guild_id,
        guild_channel,
        db,
        game_mode_target,
        *current_user_id,
    )
    .await
}

pub async fn join_helper(
    ctx: &Context,
    guild_id: GuildId,
    guild_channel: GuildChannel,
    db: Database,
    target_game_modes: IntendedGameMode,
    user_to_add: u64,
) -> anyhow::Result<String> {
    let game_mode_label = match target_game_modes {
        IntendedGameMode::Single(desired_game_mode) => desired_game_mode,
        IntendedGameMode::All => {
            // !FIXME: for "all" game modes, check all queues' occupancy - if
            // multiple will fill, respond:
            // Specify which game mode to join. XX | YY | ZZ only need one more player and
            // you cannot fill multiple game modes at once.

            return Ok("The ability to join multiple queues at once is not yet ready".to_string());

            // in here, all game mode queues will be checked - if there is one which
            // will fill, assign pass it label along, so the rest of this functions handles it.
        }
    };

    let maybe_game_mode = find_game_mode(db.clone(), &game_mode_label).await?;
    if maybe_game_mode.is_none() {
        return Ok("No game mode found with this name".to_string());
    }
    let game_mode = maybe_game_mode.unwrap();

    let mut all_queues = db::read::get_all_queues(db.clone()).await?;
    let queue = all_queues.get_mut(&game_mode).unwrap();

    // let mut queue = get_game_mode_queue(db.clone(), &game_mode.label).await?;
    let user_is_in_queue = queue
        .iter()
        .any(|join_record| join_record.player_user_id as u64 == user_to_add);

    if user_is_in_queue {
        return Ok("User is already in the queue".to_string());
    }

    let queue_not_yet_filled = queue.len() as u64 + 1 < game_mode.player_count as u64;

    if queue_not_yet_filled {
        // add player to game mode queue and exit
        queue.push(
            add_player_to_game_mode_queue(db.clone(), &game_mode.label, &user_to_add)
                .await
                .context(format!(
                    "Failed to add user {} to {} game mode",
                    &user_to_add, &game_mode.label
                ))?
                .unwrap(),
        );

        let mut users_in_queue = Vec::default();
        for join_record in queue.iter() {
            users_in_queue.push(transform::join_record_to_player_info(&ctx, join_record).await?);
        }

        let queue_names = users_in_queue
            .iter()
            .format_with(" :small_blue_diamond: ", |player_info, f| f(player_info));

        let response = MessageBuilder::new()
            .push_line("Successfully added to the waiting queue")
            .push_bold(game_mode.label)
            .push(format!(" ({}/{}) ", queue.len(), game_mode.player_count))
            .push(queue_names.to_string())
            .build();

        return Ok(response);
    }

    let mut players = queue
        .clone()
        .iter_mut()
        .map(|j| j.player_user_id as u64)
        .collect::<Vec<u64>>();
    // no need to insert this user into the queue
    // at the database level as it'll soon be cleared
    players.push(user_to_add.clone());

    let mut announcement = MessageBuilder::default();
    announcement
        .push_bold(&game_mode.label)
        .push_line(" filled!");
    for player in players.iter() {
        announcement.mention(&UserId::from(*player)).push(" ");
    }

    let m = guild_channel.say(&ctx.http, announcement.build()).await?;

    let now = Utc::now();
    let pug_thread = guild_channel
        .create_thread(
            &ctx.http,
            CreateThread::new(format!(
                "{} | {}-{}-{}",
                &game_mode.label,
                now.year(),
                now.month(),
                now.day()
            ))
            .kind(ChannelType::PublicThread)
            .auto_archive_duration(serenity::all::AutoArchiveDuration::OneHour),
        )
        .await?;

    let _working_in_thread = pug_thread.clone().start_typing(&ctx.http);

    // generate a pick sequence
    let pick_sequence = crate::utils::pick_sequence::generate(&(game_mode.player_count as u64));

    // remove participants from all queues
    db::write::remove_players_from_all_queues(db.clone(), &players)
        .await
        .context("A pug filled and the db request to remove participants from all queues failed")?;

    if game_mode.player_count == 2 {
        // two-player game modes do not undergo a picking process,
        // so we simply register a completed pug:

        let autocompleted_picking_session = PickingSession {
            created: now,
            game_mode: game_mode.label.clone(),
            thread_channel_id: pug_thread.id.get() as i64,
            pick_sequence,
            last_reset: None,
        };

        // players assigned to random team,
        // with empty team lists
        let (first_random_player, remaining_player) = match rand::Rng::gen(&mut rand::thread_rng())
        {
            true => (players.last(), players.first()),
            false => (players.first(), players.last()),
        };

        let completed_pug = transform::resolve_to_completed_pug(
            &ctx,
            db.clone(),
            autocompleted_picking_session,
            *first_random_player.unwrap(),
            vec![],
            *remaining_player.unwrap(),
            vec![],
        )
        .await
        .context(
            "Failed to auto promote 2 player game mode to completed pug \
            status after bypassing picking session",
        )?;

        // Unwrapping like this is probably fine because it comes from a String
        // (which came from a proper u64) that has not been moved about or tampered with.
        let red_player = UserId::from(completed_pug.red_team_captain as u64);
        let blue_player = UserId::from(completed_pug.blue_team_captain as u64);

        // then announce auto-picked team colors in pug thread
        let response = MessageBuilder::new()
            .push_line("Randomly assigned team colors:")
            .push("Red 🔴: ")
            .mention(&red_player)
            .push_line("")
            .push("Blue 🔵: ")
            .mention(&blue_player)
            .build();

        pug_thread.say(&ctx.http, response).await?;
    } else {
        // write picking session with these players in it
        register_picking_session(
            db.clone(),
            &pug_thread.id.get(),
            &game_mode.label,
            &players,
            pick_sequence,
        )
        .await?;

        // create commands (and register in db): /captain /autocaptain /nocapt /reset

        // TODO: perhaps it's a good idea to manually handle the error case here
        // i.e. attempt to delete any commands created so far?
        let autocaptain_cmd = guild_id
            .create_command(&ctx.http, build_autocaptain())
            .await?;
        let captain_cmd = guild_id.create_command(&ctx.http, build_captain()).await?;
        let nocaptain_cmd = guild_id
            .create_command(&ctx.http, build_nocaptain())
            .await?;
        let reset_cmd = guild_id.create_command(&ctx.http, build_reset()).await?;

        save_guild_commands(
            db.clone(),
            vec![autocaptain_cmd, captain_cmd, nocaptain_cmd, reset_cmd],
        )
        .await?;

        // spawn a timer which will auto pick captains if necessary
        let ctx_clone = ctx.clone();
        tokio::spawn(captain::autopick_countdown(
            ctx_clone,
            db.clone(),
            pug_thread.id,
            guild_id,
        ));
    }

    // TODO: announce participants' removal from queues
    // let mut announcement = MessageBuilder::default();

    return Ok(
        "If any of the following users were in the queue of any other game mode, \
    you have been removed"
            .to_string(),
    );
}

/// Remove user from game queue. Currently, this will NOT cancel a picking session if
/// the user was in one.
pub async fn leave(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<String> {
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
        Channel::Guild(channel) => match channel.kind {
            ChannelType::Text | ChannelType::PublicThread => channel,
            _ => return Ok("You cannot use this command here".to_string()),
        },
        _ => return Ok("You cannot use this command here".to_string()),
    };

    let game_modes_to_leave = match &interaction
        .data
        .options
        .iter()
        .find(|option| option.name.eq("game_mode"))
    {
        Some(game_mode_option) => {
            let arg = game_mode_option
                .value
                .as_str()
                .context("Somehow, the value of the `game_mode` option is not a string")?
                .to_string();
            IntendedGameMode::Single(arg)
        }
        None => IntendedGameMode::All,
    };

    super::queue::leave_helper(
        &ctx,
        guild_id,
        guild_channel,
        db,
        game_modes_to_leave,
        interaction.user.id.get(),
    )
    .await
}

pub async fn leave_helper(
    ctx: &Context,
    _guild_id: GuildId,
    _guild_channel: GuildChannel,
    db: Database,
    target_game_modes: IntendedGameMode,
    user_to_remove: u64,
) -> anyhow::Result<String> {
    // check for picking session which include the user
    // if found
    // - remove all captains
    // - remove all picks
    // - return everyone except the specified user to the game mode's queue, also removing
    // (and informing) those who were in the queue

    let game_mode_label = match target_game_modes {
        IntendedGameMode::Single(desired_game_mode) => desired_game_mode,
        IntendedGameMode::All => {
            return Ok("The ability to leave multiple queues at once is not yet ready".to_string());
        }
    };

    let name_of_user = match UserId::from(user_to_remove).to_user_cached(&ctx.cache) {
        Some(user) => user.name.clone(),
        None => "User".to_string(),
    };
    match db::write::remove_player_from_game_mode_queue(db, game_mode_label, user_to_remove).await?
    {
        Some(removed_join_record) => Ok(format!(
            "{} removed from {}",
            name_of_user, removed_join_record.game_mode_label
        )),
        None => Ok(format!("{} is not in the queue", name_of_user)),
    }
}

/// Show available game modes and queued players.
pub async fn list(ctx: &Context, interaction: &CommandInteraction) -> anyhow::Result<String> {
    // TODO: ensure guild channel
    let guild_id = interaction.guild_id.unwrap();

    let client = {
        let data = ctx.data.read().await;
        data.get::<DbClientRef>()
            .expect("Expected MongoDB's `Client` to be available for use")
            .clone()
    };
    let db = client.database(&guild_id.to_string());

    let mut queues: HashMap<GameMode, Vec<GameModeJoin>> = db::read::get_all_queues(db.clone())
        .await
        .context("Tried to get all queues for listing")?;

    let mut response = MessageBuilder::default();

    for (game_mode, game_mode_queue) in queues.drain() {
        if game_mode_queue.is_empty() {
            // Don't clutter the output by listing empty queues
            continue;
        }
        let mut participant_data = Vec::default();
        for join_record in game_mode_queue.iter() {
            participant_data.push(transform::join_record_to_player_info(&ctx, &join_record).await?);
        }

        let formatted_names = participant_data
            .iter()
            .format_with(" :small_blue_diamond: ", |player_info, f| f(player_info));

        response.push_line(format!(
            "**{}** ({}/{}): {}",
            game_mode.label,
            game_mode_queue.len(),
            game_mode.player_count,
            formatted_names
        ));
    }

    let response_text: String = response.build();
    if response_text.is_empty() {
        Ok("All pug queues are empty".to_string())
    } else {
        Ok(response_text)
    }
}

// if `verbose` argument is true, this player output text
// contains names in addition to player counts
async fn list_helper(
    ctx: &Context,
    queues: &HashMap<String, Vec<GameModeJoin>>,
    verbose: bool,
) -> String {
    "".to_string()
}
