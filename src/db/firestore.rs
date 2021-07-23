use super::{
    client::{ClientError, DbClient},
    jwt::ASSERTION_TARGET,
};
use crate::db::jwt::{generate, read_credentials_from_file};
use async_trait::async_trait;

use chrono::{DateTime, Duration, Utc};
use reqwest::{header, Client};
use serde::Deserialize;
use serenity::model::id::{ChannelId, GuildId};
use tracing::info;

const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";
// FIXME: move this url to .env file
const DB_ROOT_URL: &str =
    "https://firestore.googleapis.com/v1/projects/ut4-hubs/databases/(default)/documents";

#[derive(Deserialize)]
struct AccessTokenRequestResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
}

pub struct Firestore {
    // expires: DateTime<Utc>,
    client: Client, // sits behind mutex, which `operations` access
                    /*
                    designated_pug_channel:,
                    registered_game_modes:,
                    pugs_waiting_to_fill:,
                    filled_pug:,
                    completed_pug:,
                    default_voice_channels:
                    */
}

#[async_trait]
impl DbClient for Firestore {
    /// Creates a [`Firestore`].
    ///
    /// use db::Client;
    /// let db_client = Firestore::create();
    /// ```
    async fn create() -> Result<Self, ClientError> {
        // TODO: make it possible to use local, fake firestore instance which does not require credentials
        let credential_path =
            std::env::var("GCP_CREDENTIAL_FILE").expect("GCP_CREDENTIAL_FILE has not been set");
        let c = read_credentials_from_file(credential_path)
            .expect("Expected credential file to exist at path given");
        let jwt = generate(&c);
        /*
        let params = [("grant_type", GRANT_TYPE), ("assertion", &jwt)];
        let mut client = Client::new();
        let response = match client.post(ASSERTION_TARGET).form(&params).send().await {
            Ok(res) => res,
            Err(err) => {
                return Err(ClientError::AccessTokenRequest(err.to_string()));
            }
        };
        let credentials_received_time = Utc::now();

        let credentials = response
            .json::<AccessTokenRequestResponse>()
            .await
            .expect("expected request response to be parsed to json");
        info!(
            "Using {} authorization {}",
            credentials.token_type, credentials.access_token
        );

        */

        let bearer_token = format!("Bearer {}", jwt);
        info!("Using Bearer {}", jwt);

        let mut headers = header::HeaderMap::new();
        // mark security-sensitive headers with `set_sensitive`.
        let mut auth_value = header::HeaderValue::from_str(bearer_token.as_str()).unwrap();
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("expected reqwest Client to be created");

        let firestore = Self {
            client,
            // expires: credentials_received_time + Duration::seconds(credentials.expires_in),
        };

        Ok(firestore)
    }

    async fn refresh(&self) {}

    async fn purge_data(&self) -> Result<(), super::client::ClientError> {
        todo!()
    }

    async fn init_db(&self) -> Result<(), super::client::ClientError> {
        todo!()
    }
}

impl Firestore {
    // Read variant (for reading just one) isn't necessary
    async fn create_designated_pug_channel(
        &self,
        guild_id: &GuildId,
        channel_id: ChannelId,
    ) -> Result<(), super::client::ClientError> {
        // use GuildId as document id
        todo!()
    }
}
