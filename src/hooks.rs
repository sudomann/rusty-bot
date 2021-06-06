use serenity::{
    framework::standard::{macros::hook, CommandError, DispatchError, Reason},
    model::channel::Message,
    prelude::*,
};
use tracing::error;

#[hook]
pub async fn dispatch_error(context: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::CheckFailed(_, reason) => match reason {
            Reason::User(info) => {
                let _ = msg.reply(&context.http, info).await;
            }
            _ => panic!("Unimplemented response for CheckFailed event"),
        },
        DispatchError::Ratelimited(info) => {
            let _ = msg.reply(&context.http, format!("{:#?}", info)).await;
        }
        DispatchError::CommandDisabled(info) => {
            let _ = msg.reply(&context.http, info).await;
        }
        DispatchError::BlockedUser => {
            let _ = msg
                .reply(
                    &context.http,
                    "I have been instructed to ignore you - new phone, who dis?",
                )
                .await;
        }
        DispatchError::BlockedGuild => {
            let _ = msg
                .reply(
                    &context.http,
                    "Oof, it appears this server is banned from using my features :O",
                )
                .await;
        }
        DispatchError::BlockedChannel => {
            let _ = msg
                .reply(&context.http, "This channel is blocked from accessing me")
                .await;
        }
        DispatchError::OnlyForDM => {
            let _ = msg
                .reply(
                    &context.http,
                    "You can only use this command by dm'ing me. Go on, don't be shy.",
                )
                .await;
        }
        DispatchError::OnlyForGuilds => {
            let _ = msg
                .reply(&context.http, "You must be in a server to use this command")
                .await;
        }
        DispatchError::OnlyForOwners => {
            let _ = msg
                .reply(
                    &context.http,
                    "Only superusers are permitted to do that. \
                Begone peasant.\n*Turns up nose in disgust*",
                )
                .await;
        }
        DispatchError::LackingRole => {
            let _ = msg
                .reply(&context.http, "You're lacking a role this command requires")
                .await;
        }
        DispatchError::LackingPermissions(info) => {
            let response = format!(
                "Ignored\nYou lack the following required permissions:\n {:#?}",
                info
            );
            let _ = msg.reply(&context.http, response).await;
        }
        DispatchError::NotEnoughArguments { min, given } => {
            let response = format!(
                "This command requires at least {} argument(s). \
            You've given {}",
                min, given
            );
            let _ = msg.reply(&context.http, response).await;
        }
        DispatchError::TooManyArguments { max, given } => {
            let response = format!(
                "This command accepts no more than {} argument(s). \
            You've given {}",
                max, given
            );
            let _ = msg.reply(&context.http, response).await;
        }
        _ => {
            let _ = msg
                .reply(
                    &context.http,
                    "Unspecified error occured while trying to dispatch this command.\
             This has been logged",
                )
                .await;

            error!("Unknown dispatch error!",);
        }
    }
}

#[hook]
pub async fn after(_: &Context, _: &Message, cmd_name: &str, error: Result<(), CommandError>) {
    //  Print out an error if it happened
    if let Err(why) = error {
        error!("Error in {}: {:?}", cmd_name, why);
    }
}

#[hook]
pub async fn unrecognised_command(_: &Context, msg: &Message, unrecognised_command_name: &str) {
    error!(
        "A user named {:?} tried to executute an unknown command: {}",
        msg.author.name, unrecognised_command_name
    );
}
