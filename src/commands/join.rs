use serenity::{
  framework::standard::{macros::command, Args, CommandResult},
  model::prelude::*,
  prelude::*,
};

#[command]
#[aliases("j", "jp")]
#[min_args(1)]
async fn join(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
  for arg in args.iter::<String>() {
    msg
      .reply(&ctx.http, &format!("Argument: {:?}", arg.unwrap()))
      .await?;
  }

  Ok(())
}
