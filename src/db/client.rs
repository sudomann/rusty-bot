use async_trait::async_trait;
use serenity::model::id::GuildId;

#[async_trait]
pub trait DbClient: Sized {
    async fn create() -> Result<Self, ClientError>;
    async fn refresh(&self);
    async fn purge_data(&self) -> Result<(), ClientError>;
    async fn init_db(&self) -> Result<(), ClientError>;
}

#[derive(Debug)]
pub enum ClientError {
    AccessTokenRequest(String),
    Init(String),
    Permission,
    Connection,
    Unknown,
}

#[async_trait(?Send)]
pub trait CreateDestroy {
    async fn save<T>(guild_id: GuildId, item: T);
    async fn delete<T>(item: T);
}

#[async_trait(?Send)]
pub trait CreateUpdateDestroy: CreateDestroy {
    async fn edit<T>(item: T);
}
