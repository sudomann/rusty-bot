pub mod model;
pub mod read;
pub mod write;
use mongodb::{options::ClientOptions, Client};
use std::env;
use tracing::{info, instrument};

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
