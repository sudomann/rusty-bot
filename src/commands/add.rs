use super::join::join;
use crate::utils::as_another::{as_another, OpFail};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[aliases("a", "addplayer")]
#[min_args(2)]
#[max_args(2)]
// TODO: admin and owner only
async fn add(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    match as_another(msg, args).await {
        Ok(data) => join(&ctx, &data.message, data.args).await,
        Err(err) => match err {
            OpFail::NoUserMention(m) | OpFail::MultipleUserMention(m) => {
                msg.reply(ctx, m).await?;
                Ok(())
            }
        },
    }
}
