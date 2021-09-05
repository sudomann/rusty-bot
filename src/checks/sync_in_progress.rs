use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::channel::Message,
    prelude::*,
};

#[check]
#[name = "GuildDataSyncInProgress"]
#[check_in_help(false)]
#[display_in_help(false)]
async fn is_guild_data_sync_in_progress(
    _: &Context,
    _msg: &Message,
    _: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    // TODO: Check check memory to see if message guild is
    // currently being synced.
    Ok(())
    /*
    Err(Reason::UserAndLog {
        user: "Please wait a little longer to use this command.\
        \nThis this server's data is currently being synced to a database."
            .to_string(),
        log: format!(
            "Command by {} - {} was was ignored because their guild's data is being synced: {:?}",
            msg.author.id, msg.author.name, msg.guild_id
        ),
    })
    */
}
