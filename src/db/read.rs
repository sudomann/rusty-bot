use super::model::*;
use futures::stream::TryStreamExt;
use mongodb::{error::Error, Database};

pub async fn get_registered_guilds(db: Database) -> Result<Vec<Guild>, Error> {
    let collection = db.collection::<Guild>("guilds");
    let cursor = collection.find(None, None).await?;
    let v: Vec<Guild> = cursor.try_collect().await?;
    Ok(v)
}
