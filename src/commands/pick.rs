use serenity::{
  framework::standard::{macros::command, Args, CommandResult},
  model::prelude::*,
  prelude::*,
};

#[command]
#[aliases("p")]
#[min_args(1)]
#[max_args(2)]
async fn pick(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
  msg.reply(&ctx.http, "Received").await?;

  Ok(())
}
