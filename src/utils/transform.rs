use std::convert::TryInto;

use anyhow::Context as AnyhowContext;
use chrono::Utc;
use mongodb::Database;
use serenity::client::Context;
use serenity::model::channel::ChannelType;
use serenity::model::id::{CommandId, GuildId, UserId};
use serenity::model::prelude::User;

use crate::db;
use crate::db::model::{CompletedPug, PickingSession, TeamVoiceChat};

/// A convenience method to transfor [`Player`]s to [`User`]s.
pub async fn players_to_users<P>(ctx: &Context, players: P) -> anyhow::Result<Vec<User>>
where
    P: IntoIterator<Item = crate::db::model::Player>,
{
    let mut players_as_users: Vec<User> = Vec::default();
    for player in players {
        let user_id = player.user_id.parse::<u64>()?;
        let user_object = UserId(user_id).to_user(&ctx).await.context(format!(
            "Failed to obtain User object for user id: {}",
            player.user_id
        ))?;
        players_as_users.push(user_object);
    }
    Ok(players_as_users)
}

// FIXME: this helper function is an undesireable result of the picking and player tracking design in db::model.Now that I understand
// there's a possibility to use mongodb functions with the rust api, it is prudent to convert the models to a monolithic
// one upon which all/most operations will operate. See notes at bottom of db::model
/// Create completed pug record from one of the following:
///
/// - A picking session for a 2 player pug (picking gets automatically completed on queue fill)
///
/// - A picking session for a 4+ player pug (and delete the pick command)
///
/// Also creates the voice channels for teams.
///
/// The intent is to simplify the called used in commiting a completed pug to the database.
/// TODO: Two-player game modes do not involve a picking session and it does not make sense that
/// one should be coerced/shoehorned (for the integrity/accuracy of stats calculated from picking history).
pub async fn resolve_to_completed_pug(
    ctx: &Context,
    db: Database,
    picking_session: PickingSession,
    channel_position: i64,
    blue_team_captain: String,
    blue_team: Vec<String>,
    red_team_captain: String,
    red_team: Vec<String>,
) -> anyhow::Result<CompletedPug> {
    let guild_id = GuildId(db.name().parse::<u64>().context(
        "Database object name could not be parsed into a u64 guild ID. \
        Database names are *always* guild IDs",
    )?);

    // delete pick command (normally exists only when the pug involves more than two players)
    if blue_team.len() > 0 || red_team.len() > 0 {
        let search_result = db::read::find_command(db.clone(), "pick")
            .await
            .context("Failed to search for a saved /pick command in database")?;

        let saved_pick_cmd = search_result.context(
            "It appears the current pug involves more than two players, \
        which means there should be a /pick command saved in the database \
        which would be used for advancing the picking session, \
        but one was not found.",
        )?;

        let pick_cmd_id = CommandId(saved_pick_cmd.command_id);

        guild_id
            .delete_application_command(&ctx.http, pick_cmd_id)
            .await
            .context(format!(
                "Attempted and failed to delete pick command in guild: {:?}",
                guild_id.name(&ctx.cache).await
            ))?;
    }

    let category = guild_id
        .create_channel(&ctx.http, |c| {
            c.kind(ChannelType::Category)
                .name(picking_session.game_mode.as_str())
                .position(channel_position.try_into().expect("Could not convert channel position from i64 to u32. \
                This should not happen, as there cannot be so many channels in a guild the count doesn't fit u32."))
        })
        .await
        .context(format!(
            "Failed to create a voice channel category for {} pug",
            picking_session.game_mode.as_str()
        ))?;

    let blue_team_voice_channel = guild_id
        .create_channel(&ctx.http, |c| {
            c.kind(ChannelType::Voice)
                .name("Blue :blue_circle:")
                .category(category.id.0)
        })
        .await
        .context(format!(
            "Failed to create a blue team voice channel for {} pug",
            picking_session.game_mode.as_str()
        ))?;

    let red_team_voice_channel = guild_id
        .create_channel(&ctx.http, |c| {
            c.kind(ChannelType::Voice)
                .name("Red :red_circle:")
                .category(category.id.0)
        })
        .await
        .context(format!(
            "Failed to create a red team voice channel for {} pug",
            picking_session.game_mode.as_str()
        ))?;

    let completed_pug = CompletedPug {
        created: Utc::now(),
        game_mode: picking_session.game_mode,
        thread_channel_id: picking_session.thread_channel_id.to_string(),
        blue_team_captain,
        blue_team,
        red_team_captain,
        red_team,
        // !FIXME: currently voice channels are created for 2 player game modes as well. They should be exempted.
        voice_chat: TeamVoiceChat {
            category_id: category.id.0.to_string(),
            blue_channel_id: blue_team_voice_channel.id.0.to_string(),
            red_channel_id: red_team_voice_channel.id.0.to_string(),
            is_deleted_from_guild_channel_list: false,
        },
    };

    db::write::register_completed_pug(db.clone(), &completed_pug)
        .await
        .context(
            "Something went wrong with db queries when trying to \
            commit a completed pug to database along with \
            deleting the picking session record",
        )?;

    Ok(completed_pug)
}
