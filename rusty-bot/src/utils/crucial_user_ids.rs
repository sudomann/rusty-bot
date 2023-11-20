use std::collections::HashSet;
use std::env;
use std::str::FromStr;

use serenity::http::Http;
use serenity::model::id::{ApplicationId, UserId};
use serenity::Error;
use tracing::{info, instrument, warn};

pub struct CrucialIds {
    bot: ApplicationId,
    superusers: HashSet<UserId>,
}

impl CrucialIds {
    pub fn get_bot(&self) -> &ApplicationId {
        &self.bot
    }

    pub fn get_superusers(&self) -> &HashSet<UserId> {
        &self.superusers
    }
}

/// Get the Bot/Application's [`ApplicationId`] along with any other [`UserId`]'s provided via the environment
/// variable `SUPERUSERS` (provided they are correctly formatted - if not, they are ignored).

#[instrument]
pub async fn obtain(http: Http) -> Result<CrucialIds, Error> {
    match http.get_current_application_info().await {
        Ok(info) => {
            let mut superusers: HashSet<UserId> = match env::var("SUPERUSERS") {
                Ok(superusers) => superusers
                    .split_terminator(',')
                    .filter_map(|id| UserId::from_str(id).ok())
                    .collect(),
                Err(_err) => {
                    warn!(
                        "SUPERUSERS was not found in the environment, \
                        so there will be no superusers aside from the owner bot/application"
                    );
                    HashSet::default()
                }
            };
            superusers.insert(info.owner.id);
            info!("Superusers: {:?}", superusers);
            Ok(CrucialIds {
                bot: info.id,
                superusers,
            })
        }
        Err(err) => Err(err),
    }
}
