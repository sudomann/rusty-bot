// use itertools::Itertools;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    //  utils::MessageBuilder,
};

#[command]
async fn last(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    msg.reply(&ctx.http, "Coming soon!").await?;
    Ok(())
}
