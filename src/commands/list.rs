use serenity::{
  framework::standard::{macros::command, Args, CommandResult},
  model::prelude::*,
  prelude::*,
};

#[command]
#[aliases("ls")]
#[min_args(1)]
#[max_args(1)]
async fn list(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
  msg.reply(&ctx.http, "TODO - copy mem, fmt").await?;

  Ok(())
}

#[command]
#[aliases("lsa")]
async fn list_all(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
  msg.reply(&ctx.http, "TODO - copy mem, fmt").await?;

  Ok(())
}
