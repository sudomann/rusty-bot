pub mod add;
pub mod captain;
pub mod coinflip;
pub mod game_mode;
pub mod help;
pub mod join;
pub mod leave;
pub mod list;
pub mod meta;
pub mod owner;
pub mod pick;
pub mod promote;
pub mod pug_channel;
pub mod remove;
pub mod reset;
pub mod teams;
pub mod voices;
use crate::checks::pug_channel::*;
use add::*;
use captain::*;
use coinflip::*;
use game_mode::*;
use join::*;
use leave::*;
use list::*;
use meta::*;
use owner::*;
use pick::*;
use promote::*;
use pug_channel::*;
use remove::*;
use reset::*;
use serenity::framework::standard::macros::group;
use teams::*;
use voices::*;
#[group]
#[commands(git, ping, tilde)]
struct General;

#[group]
#[only_in("guilds")]
#[commands(
    captain,
    join,
    leave,
    leave_all,
    list,
    list_all,
    pick,
    promote,
    teams,
    // tag
    voices,
)]
#[checks(PugChannel)]
struct Pugs;

#[group]
#[only_in("guilds")]
#[commands(coinflip)]
struct Gambling;

#[group]
#[only_in("guilds")]
struct Stats;

#[group]
#[only_in("guilds")]
#[commands(
    pug_channel_set,
    register_game_mode,
    delete_game_mode,
    set_blue_team_default_voice_channel,
    set_red_team_default_voice_channel,
    add,
    remove,
    reset,
    random_captains
)]
struct Moderation; // pugban, pugunban, etc.

#[group]
#[owners_only]
#[commands(set_activity, quit)]
struct SuperUser;
