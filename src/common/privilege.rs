use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::prelude::*,
    prelude::*,
    // utils::MessageBuilder,
};
use std::env;

// impl From<u64> for UserId   to get kurgan's userId

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
// TODO: explicitely include kurrgan
pub async fn is_guild_admin_check(
    _ctx: &Context,
    _msg: &Message,
    _args: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    match env::var("KURRGAN_ID") {
        // not required
        Ok(_val) => {}
        Err(_e) => {}
    }
    Ok(())
}
