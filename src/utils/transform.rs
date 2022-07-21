use std::fmt;

use anyhow::Context as AnyhowContext;
use chrono::{DateTime, Utc};
use mongodb::Database;
use serenity::client::Context;
use serenity::model::channel::ChannelType;
use serenity::model::id::{ChannelId, CommandId, GuildId, UserId};
use serenity::model::prelude::User;

use crate::db;
use crate::db::model::{ChannelState, CompletedPug, GameModeJoin, PickingSession, TeamVoiceChat};

use super::time::{Accuracy, HumanTime, Tense};

/// A convenience method to transfor [`Player`]s to [`User`]s.
pub async fn players_to_users<P>(ctx: &Context, players: P) -> anyhow::Result<Vec<User>>
where
    P: IntoIterator<Item = crate::db::model::Player>,
{
    let mut players_as_users: Vec<User> = Vec::default();
    for player in players {
        let user_id = player.user_id as u64;
        let user_object = UserId(user_id).to_user(&ctx).await.context(format!(
            "Failed to obtain User object for user id: {}",
            player.user_id
        ))?;
        players_as_users.push(user_object);
    }
    Ok(players_as_users)
}

pub struct QueuedPlayerInfo {
    pub name: String,
    pub joined: DateTime<Utc>,
}

impl QueuedPlayerInfo {
    pub fn time_elapsed_since_join(&self) -> String {
        let ht = HumanTime::from(self.joined);
        ht.to_text_en(Accuracy::RoughShort, Tense::Present)
    }
}

impl fmt::Display for QueuedPlayerInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} [{}]", self.name, self.time_elapsed_since_join())
    }
}

pub async fn join_record_to_player_info(
    ctx: &Context,
    join_record: &GameModeJoin,
) -> anyhow::Result<QueuedPlayerInfo> {
    let player_user_id = UserId(join_record.player_user_id as u64);
    let player_as_user = player_user_id
        .to_user(&ctx)
        .await
        .context("Error encountered while converting `UserId` to `User`")?;
    Ok(QueuedPlayerInfo {
        name: player_as_user.name,
        joined: join_record.joined,
    })
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
/// The intent is to simplify the call used in commiting a completed pug to the database.
/// TODO: Two-player game modes do not involve a picking session and it does not make sense that
/// one should be coerced/shoehorned (for the integrity/accuracy of stats calculated from picking history).
pub async fn resolve_to_completed_pug(
    ctx: &Context,
    db: Database,
    picking_session: PickingSession,
    blue_team_captain: u64,
    mut blue_team: Vec<u64>,
    red_team_captain: u64,
    mut red_team: Vec<u64>,
) -> anyhow::Result<CompletedPug> {
    let guild_id = GuildId(db.name().parse::<u64>().context(
        "Database object name could not be parsed into a u64 guild ID. \
        Database names are *always* guild IDs",
    )?);

    // delete pick and teams commands, which (normally exist only
    // when the pug involves more than two players)
    if blue_team.len() > 0 || red_team.len() > 0 {
        let pick_cmd_search_result = db::read::find_command(db.clone(), "pick")
            .await
            .context("Failed to search for a saved /pick command in database")?;

        let saved_pick_cmd = pick_cmd_search_result.context(
            "It appears the current pug involves more than two players, \
        which means there should be a /pick command saved in the database \
        which would be used for advancing the picking session, \
        but one was not found.",
        )?;

        let pick_cmd_id = CommandId(saved_pick_cmd.command_id as u64);

        guild_id
            .delete_application_command(&ctx.http, pick_cmd_id)
            .await
            .context(format!(
                "Attempted and failed to delete pick command in guild: {:?}",
                guild_id.name(&ctx.cache)
            ))?;

        let teams_cmd_search_result = db::read::find_command(db.clone(), "teams")
            .await
            .context("Failed to search for a saved /teams command in database")?;

        let saved_teams_cmd = teams_cmd_search_result.context(
            "It appears the current pug involves more than two players, \
        which means there should be a /teams command saved in the database, \
        but one was not found.",
        )?;
        let teams_cmd_id = CommandId(saved_teams_cmd.command_id as u64);
        guild_id
            .delete_application_command(&ctx.http, teams_cmd_id)
            .await
            .context(format!(
                "Attempted and failed to delete teams command in guild: {:?}",
                guild_id.name(&ctx.cache)
            ))?;

        let reset_cmd_search_result = db::read::find_command(db.clone(), "reset")
            .await
            .context("Failed to search for a saved /reset command in database")?;

        let saved_reset_cmd = reset_cmd_search_result.context(
            "Since there was a picking session, there should be a /reset command saved in the database \
            for resetting the picking session but one was not found.",
        )?;
        let reset_cmd_id = CommandId(saved_reset_cmd.command_id as u64);
        guild_id
            .delete_application_command(&ctx.http, reset_cmd_id)
            .await
            .context(format!(
                "Attempted and failed to delete reset command in guild: {:?}",
                guild_id.name(&ctx.cache)
            ))?;

        db::write::find_and_delete_guild_commands(db.clone(), vec!["teams", "reset", "pick"])
            .await
            .context("There was an issue when trying to delete /teams, /reset and /pick commands from the database")?;
    }

    let thread_channel_id = ChannelId(picking_session.thread_channel_id as u64);

    let thread_channel = thread_channel_id
        .to_channel(&ctx)
        .await
        .context("Failed to upgrade a ChannelId to Channel")?;

    let channel_position = thread_channel.position().unwrap() + 1;
    tracing::info!("picking_session.thread_channel_id: {}", channel_position);

    let category = guild_id
        .create_channel(&ctx.http, |c| {
            c.kind(ChannelType::Category)
                .name(picking_session.game_mode.as_str())
                .position(0)
            //.position(channel_position.try_into().expect("Could not convert channel position from i64 to u32. \
            //This should not happen, as there cannot be so many channels in a guild the count doesn't fit u32."))
        })
        .await
        .context(format!(
            "Failed to create a voice channel category for {} pug",
            picking_session.game_mode.as_str()
        ))?;

    let blue_team_voice_channel = guild_id
        .create_channel(&ctx.http, |c| {
            c.kind(ChannelType::Voice)
                .name("Blue 🔵")
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
                .name("Red 🔴")
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
        thread_channel_id: picking_session.thread_channel_id,
        blue_team_captain: blue_team_captain as i64,
        blue_team: blue_team.iter_mut().map(|player_id| *player_id as i64).collect(),
        red_team_captain: red_team_captain as i64,
        red_team: red_team.iter_mut().map(|player_id| *player_id as i64).collect(),
        // !FIXME: currently voice channels are created for 2 player game modes as well. They should be exempted.
        voice_chat: TeamVoiceChat {
            category: ChannelState {
                id: category.id.0 as i64,
                is_deleted_from_guild_channel_list: false,
            },
            blue_channel: ChannelState {
                id: blue_team_voice_channel.id.0 as i64,
                is_deleted_from_guild_channel_list: false,
            },
            red_channel: ChannelState {
                id: red_team_voice_channel.id.0 as i64,
                is_deleted_from_guild_channel_list: false,
            },
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
