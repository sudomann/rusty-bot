use serenity::{
  framework::standard::{macros::command, Args, CommandResult},
  model::prelude::*,
  prelude::*,
};

#[command]
#[aliases("l", "lv")]
#[min_args(1)]
async fn leave(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
  msg.reply(&ctx.http, "Received").await?;

  Ok(())
}

#[command]
#[aliases("lva", "leaveall")]
async fn leave_all(ctx: &Context, msg: &Message) -> CommandResult {
  msg.reply(&ctx.http, "Received").await?;

  Ok(())
}
