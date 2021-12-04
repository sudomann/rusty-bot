use anyhow::Context as AnyhowContext;
use serenity::client::Context;
use serenity::model::id::UserId;
use serenity::model::prelude::User;

pub async fn players_to_users<P>(ctx: &Context, players: P) -> anyhow::Result<Vec<User>>
where
    P: IntoIterator<Item = crate::db::model::Player>,
{
    let mut players_as_users: Vec<User> = Vec::default();
    for player in players {
        let u = UserId(player.user_id).to_user(&ctx).await.context(format!(
            "Failed to obtain User object for user id: {}",
            player.user_id
        ))?;
        players_as_users.push(u);
    }
    Ok(players_as_users)
}
