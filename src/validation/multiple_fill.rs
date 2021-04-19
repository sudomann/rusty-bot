use crate::{
    pug::{GameMode, Pug},
    PugsWaitingToFill, RegisteredGameModes,
};
use serenity::{
    framework::standard::{macros::check, Args, CommandOptions, Reason},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};
use std::collections::HashSet;

#[check]
#[name = "MultipleFill"]
// Checks whether a [`join`] will result in multiple [`PugsWaitingToFill`]
// reaching capacity. Players are not allowed to fill more than one pug at a time,
pub async fn is_filling_more_than_one_pug_check(
    ctx: &Context,
    msg: &Message,
    args: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    if let Some(guild_id) = msg.guild_id {
        // TODO: do this in .before() hook if possible
        args.trimmed().quoted(); // trim whitespace and discard surrounding quotations
        let fail: Option<String> = {
            let data_read = ctx.data.read().await;

            let lock_for_registered_game_modes = data_read
                .get::<RegisteredGameModes>()
                .expect("Expected RegisteredGameModes in TypeMap")
                .clone();
            let registered_game_modes = lock_for_registered_game_modes.read().await;

            let mut game_modes: HashSet<&GameMode> = HashSet::new();
            if let Some(game_modes_in_guild) = registered_game_modes.get(&guild_id) {
                let game_modes_to_join = args
                    .clone()
                    .iter::<String>()
                    .filter(|arg| arg.is_ok())
                    // coerce the user's game mode argument(s) to lowercase,
                    // because it will be compared to lowercased game label (GameMode.key)
                    .map(|arg| arg.unwrap().to_lowercase())
                    .collect::<HashSet<String>>();
                game_modes = game_modes_in_guild
                    .iter()
                    .filter(|game_mode| game_modes_to_join.contains(game_mode.key()))
                    .collect();
            };

            let lock_for_pugs_waiting_to_fill = data_read
                .get::<PugsWaitingToFill>()
                .expect("Expected PugsWaitingToFill in TypeMap")
                .clone();
            let pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.read().await;

            match pugs_waiting_to_fill.get(&guild_id) {
                Some(potential_pugs) => {
                    let mut game_modes_which_will_fill = Vec::default();
                    for game_mode in game_modes.iter() {
                        if let Some(pug_instances_for_game_mode) = potential_pugs.get(&game_mode) {
                            if let Some(current_pug_for_game_mode) =
                                pug_instances_for_game_mode.last()
                            {
                                match current_pug_for_game_mode {
                                    Pug::Empty => continue,
                                    Pug::Players(players_in_pug) => {
                                        let current_pug_will_fill_if_joined =
                                            game_mode.capacity() - players_in_pug.len() as u8 == 1;
                                        if current_pug_will_fill_if_joined {
                                            game_modes_which_will_fill.push(game_mode);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if game_modes_which_will_fill.len() > 1 {
                        Some(
              MessageBuilder::new()
                .push("Ignored\n")
                .push("You may not fill more than one pug at a time\n")
                .push("More than one of the game modes you tried to join will fill:\n")
                .push(format!("{:?}", game_modes_which_will_fill))
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
