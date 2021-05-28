use serenity::{
    framework::standard::{macros::hook, DispatchError, Reason},
    model::channel::Message,
    prelude::*,
};

#[hook]
pub async fn dispatch_error_hook(context: &Context, msg: &Message, error: DispatchError) {
    // TODO: convert `if let` to `match` when handling the other errors
    // such as `DispatchError::LackOfPermissions`, etc.
    if let DispatchError::CheckFailed(_, reason) = error {
        match reason {
            Reason::User(info) => {
                msg.reply(&context.http, &info)
                    .await
                    .expect("Expected informational string about the failed check");
                return;
            }
            _ => panic!("Unimplemented response for CheckFailed event"),
        }
    }
}
