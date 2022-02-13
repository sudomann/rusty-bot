pub mod command_builder;
pub mod commands;
pub mod db;
pub mod error;
mod event_handler;
pub mod interaction_handlers;
pub mod jobs;
pub mod utils;
use std::env;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use event_handler::Handler;
use serenity::client::bridge::gateway::ShardManager;
use serenity::http::Http;
use serenity::prelude::*;
use tracing::error;
use tracing::log::info;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use utils::crucial_user_ids;

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

/// An object to use in reading/writing to/from the database.
///
/// From [docs](https://docs.rs/mongodb/2.0.0/mongodb/struct.Client.html):
///
/// "[`mongodb::Client`] uses [`std::sync::Arc`] internally, so it can safely be shared across threads or async tasks."
///
/// Thus we do not wrap this with [`Arc`] and [`Mutex`]/[`RwLock`], instead retrieving and cloning
/// in all threads/functions where database operations are necessary.
pub struct DbClientRef;
impl TypeMapKey for DbClientRef {
    type Value = mongodb::Client;
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

    let mut discord_client = Client::builder(&token)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
        })
        .application_id(*important_user_ids.get_bot().as_u64())
        .await
        .expect("Error creating client");
    {
        let mut data = discord_client.data.write().await;
        data.insert::<ShardManagerContainer>(discord_client.shard_manager.clone());
    }

    let shard_manager = discord_client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    info! {"Connecting to database...\n"};
    match handle_to_db_client_setup.await {
        Ok(db_client) => {
            info!("The MongoDB client connection to the database deployment is live");
            let mut data = discord_client.data.write().await;
            data.insert::<DbClientRef>(db_client);
        }
        Err(err) => {
            if err.is_panic() {
                // TODO: does this actually halt the bot during stack unwinding?
                // Resume the panic on the main task
                // panic::resume_unwind(err.into_panic());
                // err.into_panic();
                error!("Failed to establish connection between client and database. Exiting...");
                std::process::exit(1);
            } else {
                // TODO: handle this case where joining thread failed for some reason other
                // than db::setup() panicking
                panic!(
                    "Failed to join the thread that was supposed to create the \
                mongodb client object. Perhaps it was cancelled?"
                );
            }
        }
    }

    info! {"Connecting to Discord...\n"};
    if let Err(why) = discord_client.start().await {
        error!("Client error: {:?}", why);
    }
}
