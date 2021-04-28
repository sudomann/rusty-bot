use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx.http, "Pong!").await?;

    Ok(())
}

#[command]
async fn git(ctx: &Context, msg: &Message) -> CommandResult {
    let mut response = MessageBuilder::new();
    response.push_italic_line("Want a look under the hood?");
    response
        .push_bold("See the magic at <https://github.com/sudomann/rusty-bot/>")
        .build();
    msg.reply(&ctx.http, response).await?;

    Ok(())
}
