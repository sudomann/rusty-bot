pub mod model;
pub mod read;
pub mod write;
use std::env;

use mongodb::options::ClientOptions;
use mongodb::Client;
use tracing::{info, instrument};

/// Default number of seconds to wait for a useable/ready MongoDB client
/// after this application's discord client is ready.
pub const DEFAULT_MONGO_READY_MAX_WAIT: u8 = 30;

pub mod collection_name {
    pub const COMMANDS: &str = "commands";
    pub const PUG_CHANNELS: &str = "pug_channel";
    pub const GAME_MODES: &str = "game_modes";
    pub const GAME_MODE_JOINS: &str = "game_mode_joins";
    pub const GAME_MODE_ROSTER: &str = "game_mode_roster";
    pub const PICKING_SESSIONS: &str = "picking_sessions";
    pub const COMPLETED_PUGS: &str = "completed_pugs";
}

/// Creates a [`mongodb::Client`] connected to the database cluster and store a client
#[instrument]
pub async fn setup() -> Client {
    info!("Launching connection to database deployment/cluster");
    let connection_string = env::var("MONGO_URI").expect("Expected MONGO_URI in the environment");

    // Parse a connection string into an options struct.
    let mut client_options = ClientOptions::parse(connection_string)
        .await
        .expect("Expected successful parsing of connection string");

    client_options.app_name = Some("Russ T Bot".to_string());

    // Try to get and return a handle to the db cluster/deployment.
    Client::with_options(client_options)
        .expect("Expected a new mongodb::Client connected to the cluster/deployment")
}
