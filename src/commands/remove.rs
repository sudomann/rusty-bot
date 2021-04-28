use super::leave::leave;
use crate::utils::add_remove_helper::{add_remove_helper, OpFail};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[aliases("del", "delplayer", "rem")]
#[min_args(2)]
#[max_args(2)]
// TODO: admin and owner only
async fn remove(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    match add_remove_helper(msg, args).await {
        Ok(data) => leave(&ctx, &data.message, data.args).await,
        Err(err) => match err {
            OpFail::NoUserMention(m) | OpFail::MultipleUserMention(m) => {
                msg.reply(ctx, m).await?;
                Ok(())
            }
        },
    }
}
