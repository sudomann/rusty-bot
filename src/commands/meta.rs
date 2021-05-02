use serenity::client::Context;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::channel::Message;
use serenity::model::gateway::Activity;
use serenity::utils::MessageBuilder;

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

#[command]
#[aliases("setactivity")]
async fn set_activity(ctx: &Context, _msg: &Message, args: Args) -> CommandResult {
    let name = args.message();
    ctx.set_activity(Activity::playing(&name)).await;

    Ok(())
}
