use serenity::{
    framework::standard::{
        help_commands, macros::help, Args, CommandGroup, CommandResult, HelpOptions,
    },
    model::{channel::Message, id::UserId},
    prelude::*,
};
use std::collections::HashSet;

#[help]
#[strikethrough_commands_tip_in_guild = "Commands with a strikethrough like ~~`this`~~ \
    are unavailable to you because of one or more of the following:\n\
    - You do not have some permission and/or role the command requires\n\
    - I did a check and it did not pass. For example, \
    is this channel permitted for use? \
    is the command dm-only (or guild-only)?"]
#[strikethrough_commands_tip_in_dm = ""]
#[lacking_conditions = "nothing"]
#[lacking_role = "strike"]
#[lacking_permissions = "strike"]
#[individual_command_tip = "You can get detailed help info for commands - for example, type `.help voices`."]
#[command_not_found_text = "Could not find: `{}`."]
#[max_levenshtein_distance(2)]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}
