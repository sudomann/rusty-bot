use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::channel::Message,
    prelude::*,
};

#[check]
#[name = "IsConnectedToVoice"]
#[display_in_help(false)]
pub async fn is_connected_to_some_voice_channel(
    ctx: &Context,
    msg: &Message,
    _args: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    match msg.guild(&ctx).await {
        Some(guild) => match guild.voice_states.get(&msg.author.id) {
            Some(voice_state) => {
                if voice_state.channel_id.is_none() {
                    return Err(Reason::User(
                        "You need to be connected a voice channel".to_string(),
                    ));
                }
            }
            None => {
                return Err(Reason::User(
                    "You need to be connected to the voice channel you want to pull players into"
                        .to_string(),
                ));
            }
        },
        None => return Err(Reason::UserAndLog {
            user: "Sorry, that didn't work because some data is missing".to_string(),
            log:
                "Tried to retrieve guild from cache and missed - wanted to check guild.voice_states"
                    .to_string(),
        }),
    };

    Ok(())
}
