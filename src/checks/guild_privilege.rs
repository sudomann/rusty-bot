use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::channel::Message,
    prelude::*,
};

#[check]
#[name = "TeamCaptainOrGuildAdmin"]

pub async fn is_pug_captain_or_guild_admin_check(
    _ctx: &Context,
    _msg: &Message,
    _args: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    Ok(())
}

#[check]
#[name = "GuildAdmin"]
pub async fn is_guild_admin_check(
    _ctx: &Context,
    _msg: &Message,
    _args: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    Ok(())
}
