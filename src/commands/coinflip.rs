use rand::Rng;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
async fn coinflip(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let is_heads: bool = rand::thread_rng().gen();
    if is_heads {
        msg.reply(ctx, "Heads").await?;
    } else {
        msg.reply(ctx, "Tails").await?;
    }
    Ok(())
}
