use anyhow::Context as AnyhowContext;
use chrono::Utc;
use serenity::client::Context;
use serenity::model::channel::ChannelType;
use serenity::model::id::UserId;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::utils::MessageBuilder;

use crate::db::model::PickingSession;
use crate::db::read::{find_game_mode, get_game_mode_queue};
use crate::db::write::{
    add_player_to_game_mode_queue, create_picking_session, register_completed_pug,
};
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
                .auto_archive_duration(60)
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
        };

        register_completed_pug(db.clone(), &autocompleted_picking_session).await?;

        // then announce auto-picked team colors
        interaction
            .channel_id
            .say(&ctx.http, "TODO: auto-picked teams")
            .await?;
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

        // spawn a timer which will auto pick captains if necessary
        pug_thread
            .say(&ctx.http, "TODO - captain countdown placeholder")
            .await?;
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
