use crate::RegisteredGameModes;
use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};
use std::collections::HashSet;

#[check]
#[name = "ValidGameMode"]
pub async fn is_registered_game_mode_check(
    ctx: &Context,
    msg: &Message,
    args: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    if let Some(guild_id) = msg.guild_id {
        // TODO: do this in .before() hook if possible
        args.trimmed().quoted(); // trim whitespace and discard surrounding quotations
        let fail: Option<String> =
            {
                let data_read = ctx.data.read().await;
                let lock_for_registered_game_modes = data_read
                    .get::<RegisteredGameModes>()
                    .expect("Expected RegisteredGameModes in TypeMap")
                    .clone();
                let registered_game_modes = lock_for_registered_game_modes.read().await;
                match registered_game_modes.get(&guild_id) {
                    Some(game_modes) => {
                        let game_mode_keys = game_modes
                            .iter()
                            .map(|game_mode| game_mode.to_string())
                            .collect::<HashSet<String>>();
                        let game_modes_to_join = args
                            .clone()
                            .iter::<String>()
                            .filter_map(|arg| arg.ok())
                            .collect::<HashSet<String>>();

                        // the values that are in self (game_modes_to_join) self
                        // but not in other (game_mode_keys)
                        let unrecogized_game_modes = game_modes_to_join
                            .difference(&game_mode_keys)
                            .collect::<HashSet<&String>>();

                        if !unrecogized_game_modes.is_empty() {
                            Some(
              MessageBuilder::new()
                .push("Ignored\n")
                .push("Please double check the unknown game mode(s) you submitted:\n")
                .push(format!("{:?}\n", unrecogized_game_modes))
                .push(format!("Allowed game modes: {:?}", game_mode_keys))
                .build(),
            )
                        } else {
                            None
                        }
                    }
                    None => Some(
                        MessageBuilder::new()
                            .push("No game modes registered. Contact admins to run `.addmod`")
                            .build(),
                    ),
                }
            };
        if let Some(response) = fail {
            Err(Reason::User(response))
        } else {
            Ok(())
        }
    } else {
        panic!("No GuildId in received message - Is client running without gateway?");
    }
}
