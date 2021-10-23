pub mod commands;
pub mod db;
mod event_handler;
pub mod interaction_handlers;
pub mod jobs;
pub mod utils;
pub mod error;
use std::env;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use event_handler::Handler;
use serenity::client::bridge::gateway::ShardManager;
use serenity::http::Http;
use serenity::prelude::*;
use tokio::task::JoinHandle;
use tracing::error;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use utils::crucial_user_ids;

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

/// A handle to the spawned task that will try connecting to a
/// database deployment/cluster while the bot starts up.
///
/// Intended for ONE TIME USE only. Insert once, and retrieve once.
///
/// When the bot is ready, the handle can be used to join the thread
/// and retrieve the [`mongodb::Client`], provided nothing went wrong.
pub struct DbClientSetupHandle;
impl TypeMapKey for DbClientSetupHandle {
    type Value = JoinHandle<mongodb::Client>;
}
/// An object to use in reading/writing to/from the database.
///
/// From [docs](https://docs.rs/mongodb/2.0.0/mongodb/struct.Database.html):
///
/// "[`mongodb::Database`] uses [`std::sync::Arc`] internally, so it can safely be shared across threads or async tasks."
///
/// Thus we do not wrap this with [`Arc`] and [`Mutex`]/[`RwLock`], instead retrieving and cloning
/// in all threads/functions where database operations are necessary.
pub struct DbRef;
impl TypeMapKey for DbRef {
    type Value = mongodb::Database;
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    let handle_to_db_client_setup = tokio::spawn(db::setup());

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Fetch bot id and superusers' ids
    let important_user_ids = crucial_user_ids::obtain(Http::new_with_token(token.as_str()))
        .await
        .expect("Could not access application info: {:?}");

    let mut client = Client::builder(&token)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
        })
        .application_id(*important_user_ids.get_bot().as_u64())
        .await
        .expect("Error creating client");
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<DbClientSetupHandle>(handle_to_db_client_setup);
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
