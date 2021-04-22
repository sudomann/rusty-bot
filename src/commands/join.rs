use crate::{
    pug::{GameMode, PickingSession, Player},
    validation::{game_mode::*, multiple_fill::*},
    FilledPug, PugsWaitingToFill, RegisteredGameModes,
};

use linked_hash_set::LinkedHashSet;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
    utils::MessageBuilder,
};
use std::collections::{HashMap, HashSet};

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

        let lock_for_filled_pugs = {
            let data_write = ctx.data.read().await;
            data_write
                .get::<FilledPug>()
                .expect("Expected FilledPug in TypeMap")
                .clone()
        };

        let registered_game_modes_in_guild = lock_for_registered_game_modes.read().await;
        // TODO: what if PugsWaitingToFill/RegisteredGameModes not available for a a particular guild?
        // i.e. `[registered_game_modes | pugs_waiting_to_fill ].get(&guild_id)` is `None`

        if let Some(registered_game_modes) = registered_game_modes_in_guild.get(&guild_id) {
            let mut pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.write().await;

            if let Some(pugs_waiting_to_fill_in_guild) = pugs_waiting_to_fill.get_mut(&guild_id) {
                let desired_game_mode_args = args
                    .iter::<String>()
                    .filter(|arg| arg.is_ok())
                    .map(|arg| arg.unwrap().to_lowercase())
                    .collect::<HashSet<String>>();

                let mut game_mode_about_to_fill: Option<&GameMode> = None;
                let matches_some_desired_game_mode =
                    |arg: &&GameMode| desired_game_mode_args.contains(arg.key());
                let owned = registered_game_modes.clone();
                let game_modes_to_join = owned.iter().filter(matches_some_desired_game_mode);

                for game_mode in game_modes_to_join {
                    if let Some(participants) = pugs_waiting_to_fill_in_guild.get(game_mode) {
                        // check occupancy of game modes the user is joining
                        // and store the first one that is going to fill
                        if game_mode.capacity() == (participants.len() as u8 + 1) {
                            game_mode_about_to_fill = Some(game_mode);
                            // stop checking because user can only fill one pug at a time
                            break;
                        }
                    }
                }

                match game_mode_about_to_fill {
                    Some(filled_game_mode) => {
                        let mut player_copy: Option<LinkedHashSet<Player>> = None;
                        // get filled game mode participants
                        if let Some(filled_pug_players) =
                            pugs_waiting_to_fill_in_guild.get(filled_game_mode)
                        {
                            // announce pug has filled
                            msg.channel_id
                                .say(
                                    &ctx.http,
                                    MessageBuilder::new()
                                        .push(format!("{:?} has filled!", filled_game_mode))
                                        .push_bold_line(format!(
                                            "TODO - ping & DM participants: {:?}",
                                            filled_pug_players
                                        )),
                                )
                                .await?;
                            player_copy = Some(filled_pug_players.clone());
                        }

                        // TODO: Notify all removed players
                        let mut _removals: HashMap<&GameMode, &UserId> = HashMap::default();

                        // then loop through all game modes:
                        // - players that are in the filled pug are removed from all other game modes
                        // - if currently evaluating the filled game mode, move all players to a PickingSession
                        for (current_game_mode, participants) in
                            pugs_waiting_to_fill_in_guild.iter_mut()
                        {
                            if current_game_mode == filled_game_mode {
                                let mut filled_pugs = lock_for_filled_pugs.write().await;

                                if let Some(filled_pugs_in_guild) = filled_pugs.get_mut(&guild_id) {
                                    let picking_session = PickingSession::new(
                                        &current_game_mode,
                                        participants.clone(),
                                    );
                                    filled_pugs_in_guild.push_back(picking_session);
                                }

                                // clear players from this pug
                                participants.clear();

                                // in picking session, there should be a reference to the announcement,
                                // which updates every second with auto captain countdown
                            } else {
                                // remove them from *other* pugs they're in
                                if let Some(ref filled_pug_players) = player_copy {
                                    for player in filled_pug_players {
                                        if participants.remove(player) {
                                            _removals
                                                .insert(current_game_mode, player.get_user_id());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        // insert user to pugs they want to join
                        for (_game_mode, participants) in pugs_waiting_to_fill_in_guild.iter_mut() {
                            // check if one of desired game modes before joining
                            participants.insert(Player::new(msg.author.id));
                        }

                        msg.reply(
                            &ctx.http,
                            MessageBuilder::new()
                                .push(format!("TODO - composition of joined pugs"))
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
