use super::join::join;
use crate::{
    checks::pug_channel::*,
    utils::as_another::{as_another, OpFail},
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[aliases("a", "addplayer")]
#[min_args(2)]
#[max_args(2)]
#[checks(PugChannel)]
// TODO: admin and owner only
/// This command lets admins/mods add one user (via mention) to a particular pug.
///
/// e.g. `.add @sudomann ctf` or `.add ctf @sudomann`
///
/// Normally, users cannot individually join a pug while their status is
/// set to `Invisible` or `Offline` but this command allows admins to bypass that
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
