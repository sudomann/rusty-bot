use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::channel::Message,
    prelude::*,
};

#[check]
#[name = "BotAdmin"]
#[display_in_help]
async fn has_bot_admin_role(
    ctx: &Context,
    msg: &Message,
    _: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    match ctx.http.get_current_application_info().await {
        Ok(info) => {
            // FIXME: only checks for bot owner, not superuser list
            if info.owner.id == msg.author.id {
                return Ok(());
            }
        }
        Err(why) => {
            return Err(Reason::UserAndLog {
                user: "An error occured when checking bot data".to_string(),
                log: format!(
                    "Tried to get owner list, but could not access application info: {:?}",
                    why
                ),
            })
        }
    };

    let guild_id = msg.guild_id.unwrap();

    match ctx.cache.guild_roles(guild_id).await {
        Some(roles) => {
            if let Some(bot_admin_role) = roles
                .values()
                .find(|role| ["pugbot-admin", "admin"].contains(&role.name.as_str()))
            {
                match msg.author.has_role(ctx, guild_id, bot_admin_role).await {
                    Ok(user_has_role) => {
                        if user_has_role {
                            Ok(())
                        } else {
                            Err(Reason::User(
                                "Ignored - You need to have at least one of the required roles - `pugbot-admin`, `admin` - for this command".to_string(),
                            ))
                        }
                    }
                    Err(err) => Err(Reason::UserAndLog {
                        user: "An error occured when checking your role(s)".to_string(),
                        log: err.to_string(),
                    }),
                }
            } else {
                Err(Reason::User(
                    "The role `pugbot-admin` does not exist in this server".to_string(),
                ))
            }
        }
        None => Err(Reason::UserAndLog {
            user: ("Sorry, something went wrong when evaluating your roles".to_string()),
            log: ("Could not get guild roles from cache".to_string()),
        }),
    }
}
