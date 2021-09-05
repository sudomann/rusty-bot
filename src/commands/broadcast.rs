use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
#[only_in("dm")]
/// DM a message to privilleged members of all guilds
///
/// For use making administrative announcements only,
/// such as taking the bot down for upgrades and maintenance
async fn broadcast(_ctx: &Context, _msg: &Message, _: Args) -> CommandResult {
    // TODO: implement this
    Ok(())
}
