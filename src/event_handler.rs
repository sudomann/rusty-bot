use crate::{
    data_structure::{
        CompletedPug, /* DbClientContainer, */ DefaultVoiceChannels, DesignatedPugChannel,
        FilledPug, PugsWaitingToFill, RegisteredGameModes,
    },
    // db::{client::DbClient, firestore::Firestore},
    jobs::start_jobs,
    pug::{
        game_mode::GameMode, picking_session::PickingSession, player::Players,
        voice_channels::TeamVoiceChannels,
    },
};
use itertools::join;
use serenity::{
    async_trait,
    model::{
        channel::{GuildChannel, Message},
        event::ResumedEvent,
        gateway::{Activity, Ready},
        guild::Guild,
        id::GuildId,
        prelude::OnlineStatus::*,
    },
    prelude::*,
    utils::MessageBuilder,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use tracing::info;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
        ctx.set_activity(Activity::playing("Bugs? Message sudomann#9568"))
            .await;
    }

    async fn cache_ready(&self, ctx: Context, guild_ids: Vec<GuildId>) {
        /*
        let db_client = match Firestore::create().await {
            Ok(firestore) => {
                // TODO
            }
            Err(err) => {
                // TODO
            }
        };
        */

        let designated_pug_channel = HashMap::default();
        let mut registered_game_modes: HashMap<GuildId, HashSet<GameMode>> = HashMap::default();
        let mut pugs_waiting_to_fill: HashMap<GuildId, HashMap<GameMode, Players>> =
            HashMap::default();
        let mut filled_pugs: HashMap<GuildId, VecDeque<PickingSession>> = HashMap::default();
        let mut completed_pugs: HashMap<GuildId, Vec<PickingSession>> = HashMap::default();
        let mut team_voice_channels: HashMap<GuildId, TeamVoiceChannels> = HashMap::default();
        let preset_gamemodes: HashSet<GameMode> = HashSet::default();

        // FIXME! remove db specific code from here
        /*  Firestore's REST API as of writing does not allow generating our own database ids as of yet.
            We have to use the default database id which is currently the following
            (glaringly literal) string:
            (default)
            And yes, you have to include the parentheses
        */
        // projects/ut4-hubs/databases/(default)/documents/CompletedPug/
        // let _firestore_db_id = std::env::var("FIRESTORE_DB_ID").unwrap_or("(default)".to_string());
        info!("Announce db being used");
        // initialize pug state data
        for guild_id in guild_ids.iter() {
            registered_game_modes.insert(*guild_id, preset_gamemodes.clone());
            let mut potential_pugs: HashMap<GameMode, Players> = HashMap::default();
            for game_mode in preset_gamemodes.clone().drain() {
                potential_pugs.insert(game_mode, Players::default());
            }
            pugs_waiting_to_fill.insert(*guild_id, potential_pugs);
            let temp_deque: VecDeque<PickingSession> = VecDeque::default();
            filled_pugs.insert(*guild_id, temp_deque);
            completed_pugs.insert(*guild_id, Vec::default());
            team_voice_channels.insert(*guild_id, TeamVoiceChannels::new(None, None));
        }

        {
            let mut data = ctx.data.write().await;
            // data.insert::<DbClientContainer>(Arc::new(Mutex::new(db_client)));
            data.insert::<DesignatedPugChannel>(Arc::new(RwLock::new(designated_pug_channel)));
            data.insert::<RegisteredGameModes>(Arc::new(RwLock::new(registered_game_modes)));
            data.insert::<PugsWaitingToFill>(Arc::new(RwLock::new(pugs_waiting_to_fill)));
            data.insert::<FilledPug>(Arc::new(RwLock::new(filled_pugs)));
            data.insert::<CompletedPug>(Arc::new(RwLock::new(completed_pugs)));
            data.insert::<DefaultVoiceChannels>(Arc::new(RwLock::new(team_voice_channels)));
        }
        start_jobs(ctx);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }

    async fn channel_create(&self, _ctx: Context, _channel: &GuildChannel) {}

    async fn category_create(
        &self,
        _ctx: Context,
        _category: &serenity::model::channel::ChannelCategory,
    ) {
    }

    async fn category_delete(
        &self,
        _ctx: Context,
        _category: &serenity::model::channel::ChannelCategory,
    ) {
        // TODO: if it contained a registered team voice channel, remove the entry from data store
    }

    async fn channel_delete(&self, _ctx: Context, _channel: &GuildChannel) {
        // TODO: if it was a registered team voice channel, remove the entry from data store
    }

    async fn guild_ban_addition(
        &self,
        _ctx: Context,
        _guild_id: GuildId,
        _banned_user: serenity::model::prelude::User,
    ) {
        // TODO: check pending pug data and remove the user giving the reason, they've just been banned
        // For past, completed pugs, maybe modify the data (i.e. User.name) to indicate somehow "BANNED"
    }

    async fn guild_create(&self, _ctx: Context, _guild: Guild, is_new: bool) {
        if is_new {
            // TODO: create data map in db and in memory
        }
    }

    async fn guild_member_addition(
        &self,
        _ctx: Context,
        _guild_id: GuildId,
        _new_member: serenity::model::guild::Member,
    ) {
    }

    async fn guild_member_removal(
        &self,
        _ctx: Context,
        _guild_id: GuildId,
        _user: serenity::model::prelude::User,
        _member_data_if_available: Option<serenity::model::guild::Member>,
    ) {
        // TODO: probably same thing guild_ban_addition() does
    }

    async fn message_update(
        &self,
        _ctx: Context,
        _old_if_available: Option<Message>,
        _new: Option<Message>,
        _event: serenity::model::event::MessageUpdateEvent,
    ) {
    }

    async fn presence_update(
        &self,
        ctx: Context,
        new_data: serenity::model::event::PresenceUpdateEvent,
    ) {
        let guild_id = match new_data.guild_id {
            Some(id) => id,
            // I think this is a presence update for a dm channel
            None => return,
        };

        let data = ctx.data.read().await;
        let designated_pug_channel_lock = data
            .get::<DesignatedPugChannel>()
            .expect("Expected DesignatedPugChannel in TypeMap");
        let designated_pug_channels = designated_pug_channel_lock.read().await;

        let pug_channel_id = match designated_pug_channels.get(&guild_id) {
            Some(channel_id) => channel_id.clone(),
            None => {
                // guild does not have a registered pug channel - no need to go any further
                return;
            }
        };

        let lock_for_unfilled_pugs = match new_data.presence.status {
            Invisible | Offline => {
                // If their presence was just updated to invisible/offline,
                // we need to check if joined any unfilled pugs and kick them
                data.get::<PugsWaitingToFill>()
                    .expect("Expected PugsWaitingToFill in TypeMap")
                    .clone()
            }
            _ => {
                /* ignore */
                return;
            }
        };

        let mut unfilled_pugs = lock_for_unfilled_pugs.write().await;

        let unfilled_pugs_in_guild = unfilled_pugs.get_mut(&guild_id).unwrap();
        let mut game_modes_removed_from: Vec<String> = Vec::default();
        let user_id = new_data.presence.user_id;
        for (game_mode, waiting_player_list) in unfilled_pugs_in_guild.iter_mut() {
            if waiting_player_list.remove(&user_id) {
                game_modes_removed_from.push(game_mode.label().to_owned());
            }
        }

        if !game_modes_removed_from.is_empty() {
            let message = MessageBuilder::new()
                .push(user_id.mention())
                .push(" you were removed from ")
                .push_line(join(game_modes_removed_from, " :small_orange_diamond: "))
                .push("because you went invisible/offline")
                .build();
            let _ = pug_channel_id.say(&ctx.http, message).await;
        }
    }
}
