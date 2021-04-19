use crate::{
    pug::{GameMode, Pug, PugParticipant},
    validation::{game_mode::*, multiple_fill::*},
    PugsWaitingToFill, RegisteredGameModes,
};
use linked_hash_set::LinkedHashSet;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};
use std::collections::HashSet;

#[command]
#[aliases("j", "jp")]
#[min_args(1)]
#[checks(ValidGameMode, MultipleFill)]
async fn join(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if let Some(guild_id) = msg.guild_id {
        let lock_for_registered_game_modes = {
            let data_read = ctx.data.read().await;

            data_read
                .get::<RegisteredGameModes>()
                .expect("Expected RegisteredGameModes in TypeMap")
                .clone()
        };

        let lock_for_pugs_waiting_to_fill = {
            let data_write = ctx.data.read().await;
            data_write
                .get::<PugsWaitingToFill>()
                .expect("Expected PugsWaitingToFill in TypeMap")
                .clone()
        };

        let registered_game_modes = lock_for_registered_game_modes.read().await;
        // TODO: what if PugsWaitingToFill/RegisteredGameModes not available for a a particular guild?
        // i.e. `[registered_game_modes | pugs_waiting_to_fill ].get(&guild_id)` is `None`

        if let Some(_game_modes) = registered_game_modes.get(&guild_id) {
            let mut pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.write().await;

            if let Some(pugs_waiting_to_fill_in_guild) = pugs_waiting_to_fill.get_mut(&guild_id) {
                let desired_game_mode_args = args
                    .iter::<String>()
                    .filter_map(|arg| arg.ok())
                    .collect::<HashSet<String>>();
                let mut game_modes_to_join: Vec<&GameMode> = Vec::default();

                for (registered_game_mode, pugs) in pugs_waiting_to_fill_in_guild.iter_mut() {
                    if desired_game_mode_args.contains(registered_game_mode.key()) {
                        // add user to pug
                        if let Some(current_pug_for_game_mode) = pugs.last_mut() {
                            let player = PugParticipant::new(msg.author.id);
                            // multiple join attempts wont result in duplicate
                            // instances of UserId in LinkedHashSet
                            match current_pug_for_game_mode {
                                Pug::Empty => {
                                    let mut new_player_set = LinkedHashSet::default();
                                    new_player_set.insert(player);
                                    let new_pug = Pug::Players(new_player_set);
                                    pugs.push(new_pug);
                                }
                                Pug::Players(players) => {
                                    players.insert(player);
                                    game_modes_to_join.push(registered_game_mode);
                                }
                            };
                        }
                        msg.reply(
                            &ctx.http,
                            MessageBuilder::new()
                                .push(format!("okay - {:?}", pugs.last()))
                                .build(),
                        )
                        .await?;
                    }
                }
            }
        } else {
            msg
              .channel_id
              .say(
                &ctx.http,
                MessageBuilder::new()
                  .push("No data found for this guild.")
                  .push("This can happen when bot is added into a guild while it's already running.")
                  .push(
                    "This data for guilds is composed during startup, so try relaunching the bot.",
                  )
                  .build(),
              )
              .await?;
        }
    }

    Ok(())
}
