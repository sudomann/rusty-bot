use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[min_args(1)]
/// The bot repeats whatever you say.
///
/// Currently it is not supported to have the bot send commands to itself.
/// Those will be echoed but not executed.
async fn echo(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    msg.reply(&ctx.http, args.rest()).await?;
    Ok(())
}
