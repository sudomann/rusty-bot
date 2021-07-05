use super::leave::leave;
use crate::{
    checks::{pug_channel::*, sync_in_progress::*},
    utils::as_another::{as_another, OpFail},
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[aliases("r", "del", "delplayer", "rm")]
#[min_args(2)]
#[max_args(2)]
#[checks(PugChannel, GuildDataSyncInProgress)]
// TODO: admin and owner only
/// This command lets admins/mods remove one user (via mention) from a particular pug .
///
/// e.g. `.remove @sudomann ctf` or `.add ctf @sudomann`
async fn remove(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    match as_another(msg, args).await {
        Ok(data) => leave(&ctx, &data.message, data.args).await,
        Err(err) => match err {
            OpFail::NoUserMention(m) | OpFail::MultipleUserMention(m) => {
                msg.reply(ctx, m).await?;
                Ok(())
            }
        },
    }
}
