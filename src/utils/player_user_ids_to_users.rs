use serenity::{
    client::Context,
    model::{id::UserId, user::User},
    prelude::SerenityError,
};

pub async fn player_user_ids_to_users(
    ctx: &Context,
    players: impl IntoIterator<Item = &(u8, UserId)>,
) -> Result<Vec<(u8, User)>, SerenityError> {
    let mut output: Vec<(u8, User)> = Vec::default();
    for (number, user_id) in players {
        let user = user_id.to_user(&ctx.http).await?;
        output.push((*number, user));
    }
    Ok(output)
}
