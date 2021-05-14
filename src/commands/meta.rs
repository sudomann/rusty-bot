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

#[command]
async fn tilde(ctx: &Context, msg: &Message) -> CommandResult {
    let message = MessageBuilder::new()
    .push_line("The tilde character `~` is sometimes used for commands on this bot.")
    .push_line("I understand this might be annoying, but it is because *sometimes* there are two versions of me, running on different machines.")
    .push_line("`But why?`, you might ask.")
    .push_line("One version of me runs on a proper server in the cloud and expect commands to start with `.` which you are more familiar with.")
    .push_line("The other version of me runs on a home computer and expects commands to start with `~`.")
    .push_line("Since both versions can run at the same time, `~` and `.` are used to specify which version should respond to the command.")
    .push_line("Sometimes my newest capabilities/features need to be tested from the latest code (that resides on a home computer).")
    .push_line("Having different command prefixes avoids both versions responding to the same command and causing confusion.")
    .build();
    msg.channel_id.say(&ctx.http, message).await?;

    Ok(())
}
