use crate::{
    db::firestore::Firestore,
    pug::{
        game_mode::GameMode, picking_session::PickingSession, player::Players,
        voice_channels::TeamVoiceChannels,
    },
};
use serenity::{
    client::bridge::gateway::ShardManager,
    model::id::{ChannelId, GuildId},
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    vec::Vec,
};
use tokio::sync::RwLock;

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct DesignatedPugChannel;
impl TypeMapKey for DesignatedPugChannel {
    type Value = Arc<RwLock<HashMap<GuildId, ChannelId>>>;
}

pub struct RegisteredGameModes;
impl TypeMapKey for RegisteredGameModes {
    type Value = Arc<RwLock<HashMap<GuildId, HashSet<GameMode>>>>;
}

pub struct PugsWaitingToFill;
impl TypeMapKey for PugsWaitingToFill {
    type Value = Arc<RwLock<HashMap<GuildId, HashMap<GameMode, Players>>>>;
}

pub struct FilledPug;
impl TypeMapKey for FilledPug {
    type Value = Arc<RwLock<HashMap<GuildId, VecDeque<PickingSession>>>>;
}

pub struct CompletedPug;
impl TypeMapKey for CompletedPug {
    type Value = Arc<RwLock<HashMap<GuildId, Vec<PickingSession>>>>;
}

pub struct DefaultVoiceChannels;
impl TypeMapKey for DefaultVoiceChannels {
    type Value = Arc<RwLock<HashMap<GuildId, TeamVoiceChannels>>>;
}
