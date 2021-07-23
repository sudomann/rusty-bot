use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::prelude::{OnlineStatus::*, *},
    prelude::*,
};

const NO_INVIS_OR_OFFLINE_MSG: &str = "Invisible/Offline players cannot participate in pugs";

#[check]
#[name = "NoInvisbleOrOfflineStatus"]
#[display_in_help(false)]
async fn is_not_invisible_or_offline_status(
    ctx: &Context,
    msg: &Message,
    _: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    let guild_id = msg.guild_id.unwrap();

    if let Some(guild) = guild_id.to_guild_cached(&ctx).await {
        // Note: with serenity 0.10.7, `guild.presences` seems to contain
        // presence data only for the users current status is Online/Idle/Do Not Disturb
        // This does not line up with what the 0.10.7 docs may lead you to believe:
        // if you have the GatewayIntents::GUILD_MEMBERS intent, then `guild.presences`
        // is populated - if you don't then it's empty
        //
        // SO, we proceed by using guild.presences.get(&msg.author.id) ---> None to conclude
        // that the user is offline, since we've enabled using the GatewayIntents::GUILD_MEMBERS intent
        let presence = guild.presences.get(&msg.author.id);
        if presence.is_none() {
            return Err(Reason::User(NO_INVIS_OR_OFFLINE_MSG.to_string()));
        }

        // Despite the behavior described above, we will still check the unwrapped presence status
        // in case there is inconsistency with the population of `guild.presences` data
        match presence.unwrap().status {
            Invisible | Offline => {
                return Err(Reason::User(NO_INVIS_OR_OFFLINE_MSG.to_string()));
            }
            _ => {}
        }
    }
    Ok(())
}
