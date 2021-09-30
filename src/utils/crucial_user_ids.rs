use serenity::{http::Http, model::id::UserId, Error};
use std::{collections::HashSet, env, str::FromStr};

pub struct CrucialUserIds {
    bot: UserId,
    superusers: HashSet<UserId>,
}

impl CrucialUserIds {
    pub fn get_bot(&self) -> &UserId {
        &self.bot
    }

    pub fn get_superusers(&self) -> &HashSet<UserId> {
        &self.superusers
    }
}

/// Get the Bot/Application's [`UserId`] along with those provided to the environment
/// variable `SUPERUSERS` (provided they are correctly formatted - if not, they are ignored).
pub async fn obtain(http: Http) -> Result<CrucialUserIds, Error> {
    match http.get_current_application_info().await {
        Ok(info) => {
            let mut superusers: HashSet<UserId> = match env::var("SUPERUSERS") {
                Ok(superusers) => {
                    let superuser_ids: HashSet<&str> = superusers.split_terminator(',').collect();
                    superuser_ids
                        .iter()
                        .filter_map(|id| UserId::from_str(id).ok())
                        .collect()
                }
                Err(_err) => {
                    // TODO: announce that there was an error parsing the provided superusers list,
                    // so there will be no superusers aside from the bot/application owner
                    HashSet::default()
                }
            };
            superusers.insert(info.owner.id);

            Ok(CrucialUserIds {
                bot: info.id,
                superusers,
            })
        }
        Err(err) => Err(err),
    }
}
