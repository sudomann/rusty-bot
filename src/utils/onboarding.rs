use std::sync::Arc;

use mongodb::error::Error;
use serenity::client::Context;
use serenity::model::id::GuildId;
use tracing::{info, instrument};

use crate::db::read::get_registered_guilds;
use crate::DbRef;

/// Check whether provideds guilds are saved in database. For those are not,
/// create a new document for them in database, then create the initial guild commands.
#[instrument(skip(ctx))]
pub async fn ensure_guild_registration(
    ctx: Arc<Context>,
    guilds: Vec<GuildId>,
) -> Result<(), Error> {
    info!("Fetching all existing guilds");

    // get all guilds registered in db and
    let db = {
        let data = ctx.data.read().await;
        data.get::<DbRef>().unwrap().clone()
    };

    let guilds = get_registered_guilds(db).await?;
    println!("{:?}", guilds);

    Ok(())
}

pub async fn create_initial_commands() {}
