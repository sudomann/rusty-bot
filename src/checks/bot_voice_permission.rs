use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::channel::Message,
    prelude::*,
};

#[check]
#[name = "BotHasVoicePermission"]
#[display_in_help(false)]
pub async fn is_bot_allowed_to_move_voice_channel_users(
    ctx: &Context,
    msg: &Message,
    _args: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    let guild_id = msg.guild_id.unwrap();
    let this_bot_user_id = ctx.cache.current_user_id().await;
    match guild_id.member(ctx, this_bot_user_id).await {
        Ok(bot_membership) => {
            let has_move_members_perm = bot_membership
                .permissions(ctx)
                .await
                .expect("Expected to get bot permissions in guild")
                .move_members();

            if !has_move_members_perm {
                return Err(Reason::User(
                    "I don't have the `Move Members` permission :( \
          Please contact an admin to fix this so I can move players"
                        .to_string(),
                ));
            };
        }
        Err(err) => {
            return Err(Reason::UserAndLog {
                user: "Something went wrong when retrieving the details of my membership in this server".to_string(),
                log: err.to_string(),
            })
        }
    };

    Ok(())
}
