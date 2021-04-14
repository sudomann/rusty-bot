use serenity::{
  framework::standard::{macros::command, Args, CommandResult},
  model::prelude::*,
  prelude::*,
  utils::MessageBuilder,
};

#[command]
#[aliases("pro")]
#[min_args(1)]
// TODO: admin configurable rate limiting (by number of invocations per minute, no matter who)
async fn promote(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
  msg
    .channel_id
    .say(
      &ctx.http,
      MessageBuilder::new()
        .push("Promoting - TODO -")
        .mention(&msg.author)
        .build(),
    )
    .await?;

  Ok(())
}
