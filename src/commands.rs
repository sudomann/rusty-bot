pub mod add;
pub mod broadcast;
pub mod captain;
pub mod coinflip;
pub mod echo;
pub mod game_mode;
pub mod help;
pub mod here;
pub mod join;
pub mod last;
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
use crate::checks::{pug_channel::*, roles::*};
use add::*;
use broadcast::*;
use captain::*;
use coinflip::*;
use echo::*;
use game_mode::*;
use here::*;
use join::*;
use last::*;
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
#[checks(PugChannel)]
#[commands(
    captain,
    here,
    join,
    last,
    leave,
    leave_all,
    list,
    list_all,
    no_captain,
    pick,
    promote,
    teams,
    // tag
    last,
    voices,
)]
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
// #[allowed_roles("admin", "pugbot-admin")] <--- unreliable - ignores owner_privilege
// replicated its behavior in BotAdmin check
#[checks(BotAdmin)]
#[owner_privilege]
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
#[commands(echo, set_activity, quit, broadcast)]
struct SuperUser;
