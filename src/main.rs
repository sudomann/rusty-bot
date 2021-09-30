pub mod commands;
pub mod db;
mod event_handler;
pub mod interaction_handlers;
pub mod jobs;
pub mod utils;
use event_handler::Handler;
use serenity::{client::bridge::gateway::ShardManager, http::Http, prelude::*};
use std::{
    env,
    sync::{atomic::AtomicBool, Arc},
};
use tracing::error;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use utils::crucial_user_ids;

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    tokio::spawn(async { db::setup() });

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
