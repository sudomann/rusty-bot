use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
async fn here(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    msg.reply(
        ctx,
        "No need. Since AFK players can be removed from pugs easily \
  using **.reset** and **.delplayer**, this command does nothing.",
    )
    .await?;
    Ok(())
}
