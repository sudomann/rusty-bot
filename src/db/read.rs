use futures::stream::TryStreamExt;
use mongodb::error::Error;
use mongodb::Database;

use super::model::*;

pub async fn get_registered_guilds(db: Database) -> Result<Vec<Guild>, Error> {
    let collection = db.collection::<Guild>("guilds");
    let cursor = collection.find(None, None).await?;
    let v: Vec<Guild> = cursor.try_collect().await?;
    Ok(v)
}
