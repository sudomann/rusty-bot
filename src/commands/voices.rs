use crate::utils::player_user_ids_to_users::*;
use itertools::Itertools;
use rand::seq::SliceRandom;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};

#[command]
#[aliases("voice", "eugene")]
#[max_args(0)]
#[allowed_roles("admin", "voice_channel_admin")]
/// Moves pug participants into their team voice channels if they are not in them already.
/// Only works on people that are already connected to voice chat
/// Uninplemented
async fn voices(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    unimplemented!()
}
