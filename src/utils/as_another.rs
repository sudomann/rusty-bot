use serenity::{
    framework::standard::{Args, Delimiter},
    model::channel::Message,
};

/// Try to get a mentioned user from provided Arg, then modify
/// the provided Message to assign the user as the author
/// Upon success, a [`OnBehalfOf`] is returned with the info
/// so commands can be invoked as the mentioned user.
/// Upon failure, the error returned contains a user-friendly help string.
pub async fn as_another(msg: &Message, mut args: Args) -> Result<OnBehalfOf, OpFail> {
    if msg.mentions.len() == 0 {
        return Err(OpFail::NoUserMention(
            "You need to mention a user".to_string(),
        ));
    }
    if msg.mentions.len() > 1 {
        return Err(OpFail::MultipleUserMention(
            "You can only mention one user".to_string(),
        ));
    }

    // At this point:
    // there is only 1 mention
    // min_args and max_args enforce 2 arguments, so
    // there are exactly 2 arguments to parse
    let first = args.single::<String>().unwrap();
    let second = args.single::<String>().unwrap();
    let new_arg = {
        if first.contains("<@") {
            Args::new(second.as_str(), &[Delimiter::Single(' ')])
        } else {
            Args::new(first.as_str(), &[Delimiter::Single(' ')])
        }
    };
    // create duplicate message and overwrite the author with the user being join()'ed to a pug
    // pop() will return the only User that is in mentions
    let mut new_msg = msg.clone();
    new_msg.author = msg.mentions.clone().pop().unwrap();
    Ok(OnBehalfOf {
        message: new_msg,
        args: new_arg,
    })
}

pub struct OnBehalfOf {
    pub message: Message,
    pub args: Args,
}

pub enum OpFail {
    NoUserMention(String),
    MultipleUserMention(String),
}
