// mod command_history;
mod checks;
mod commands;
mod data_structure;
mod db;
mod event_handler;
mod hooks;
mod jobs;
mod pug;
mod utils;
use commands::{help::*, *};
use data_structure::ShardManagerContainer;
use event_handler::Handler;
use hooks::{after_hook, dispatch_error_hook, unrecognised_command_hook};
use pug::voice_channels::TeamVoiceChannels;
use serenity::{framework::standard::StandardFramework, http::Http, model::id::UserId, prelude::*};
use std::{collections::HashSet, env, str::FromStr};
use tracing::error;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");

    // Initialize the logger using environment variables.
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);

    // Fetch bot owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners: HashSet<UserId> = match env::var("SUPERUSERS") {
                Ok(superusers) => {
                    let superuser_ids: HashSet<&str> = superusers.split_terminator(',').collect();
                    superuser_ids
                        .iter()
                        .filter_map(|id| UserId::from_str(id).ok())
                        .collect()
                }
                Err(_err) => HashSet::default(),
            };
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let environment = env::var("ENV").expect("Expected 'ENV' in environment");
    let prefix = match environment.as_str() {
        "PROD" => ("."),
        _ => ("~"),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| {
            c.with_whitespace(true)
                .case_insensitivity(true)
                .owners(owners)
                .prefix(prefix)
        })
        .on_dispatch_error(dispatch_error_hook)
        .after(after_hook)
        .unrecognised_command(unrecognised_command_hook)
        .help(&MY_HELP)
        .group(&GENERAL_GROUP)
        .group(&PUGS_GROUP)
        .group(&GAMBLING_GROUP)
        .group(&MODERATION_GROUP)
        .group(&STATS_GROUP)
        .group(&SUPERUSER_GROUP);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();

    // TODO: attach perhaps a handler to announce to admins that bot is going offline
    // How to pass reason? Distinguish SIGINT and SIGTERM
    // Neglect Windows support :/
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
