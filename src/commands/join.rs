use crate::{
    pug::{
        game_mode::GameMode,
        picking_session::{PickingSession, SetCaptainSuccess},
        player::Player,
    },
    utils::parse_game_modes::{parse_game_modes, GameModeError},
    FilledPug, PugsWaitingToFill,
};
use itertools::Itertools;
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
pub async fn join(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if let Some(guild_id) = msg.guild_id {
        let lock_for_pugs_waiting_to_fill = {
            let data_read = ctx.data.read().await;
            data_read
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

        // TODO: what if PugsWaitingToFill not available for a a particular guild?
        // i.e. `[registered_game_modes | pugs_waiting_to_fill ].get(&guild_id)` is `None`

        let registered_game_modes = match parse_game_modes(ctx, guild_id, args.clone()).await {
            Ok(game_modes) => game_modes,
            Err(err) => {
                match err {
                    GameModeError::NoneGiven(m)
                    | GameModeError::NoneRegistered(m)
                    | GameModeError::Foreign(m) => {
                        msg.reply(ctx, m).await?;
                    }
                }
                return Ok(());
            }
        };

        let mut pugs_waiting_to_fill = lock_for_pugs_waiting_to_fill.write().await;

        if let Some(pugs_waiting_to_fill_in_guild) = pugs_waiting_to_fill.get_mut(&guild_id) {
            let mut game_modes_which_will_fill = Vec::default();
            for game_mode in registered_game_modes.iter() {
                if let Some(participants) = pugs_waiting_to_fill_in_guild.get(game_mode) {
                    if participants.is_empty() {
                        continue;
                    } else {
                        let will_fill_on_join =
                            game_mode.capacity() - participants.len() as u8 == 1;
                        if will_fill_on_join {
                            game_modes_which_will_fill.push(game_mode);
                        }
                    }
                }
            }

            if game_modes_which_will_fill.len() > 1 {
                let response = MessageBuilder::new()
                    .push("Ignored\n")
                    .push("You may not fill more than one pug at a time\n")
                    .push("More than one of the game modes you tried to join will fill:\n")
                    .push(format!("{:?}", game_modes_which_will_fill))
                    .build();
                msg.reply(ctx, response).await?;
                return Ok(());
            }

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

            if game_mode_about_to_fill.is_none() {
                let mut response = MessageBuilder::new();
                // insert user to pugs they want to join
                for (game_mode, participants) in pugs_waiting_to_fill_in_guild.iter_mut() {
                    // check if one of desired game modes before joining
                    if desired_game_mode_args.contains(game_mode.label()) {
                        participants.insert(Player::new(msg.author.clone()));

                        let participants_text = participants.iter().format_with(
                            " :small_orange_diamond: ",
                            |player, f| {
                                f(&format_args!(
                                    "{}[{}]",
                                    player.get_user().name,
                                    player.time_elapsed_since_join().num_minutes() // TODO: better time elapsed formatting
                                ))
                            },
                        );
                        response.push_bold_line(format!(
                            "__{} [{}/{}]:__",
                            game_mode.label(),
                            participants.len(),
                            game_mode.capacity()
                        ));
                        response.push_line(participants_text).build();
                    }
                }

                msg.channel_id.say(&ctx.http, response).await?;
                return Ok(());
            }

            let filled_game_mode = game_mode_about_to_fill.unwrap();
            let mut player_copy: Option<LinkedHashSet<Player>> = None;
            // get filled game mode participants
            if let Some(existing_players) = pugs_waiting_to_fill_in_guild.get_mut(filled_game_mode)
            {
                // try to add current user
                let is_already_in = !existing_players.insert(Player::new(msg.author.clone()));
                if is_already_in {
                    msg.channel_id.say(&ctx.http, "You already joined").await?;
                    return Ok(());
                }
                // compose filled pug announcement and dm
                let participants_text = existing_players
                    .iter()
                    .format_with(" :small_orange_diamond: ", |player, f| {
                        f(&format_args!("{}", player.get_user().mention()))
                    });
                let participants_text_for_dm = existing_players
                    .iter()
                    .format_with(" :small_orange_diamond: ", |player, f| {
                        f(&format_args!("{}", player.get_user().name))
                    });
                let notice = format!("{} has been filled: ", filled_game_mode.label());
                let mut dm_announcement = MessageBuilder::new();
                // this can fail if the bot does not have create invite permission in a guild
                dm_announcement
                    .push_line(notice.clone())
                    .push_line(participants_text_for_dm);
                let invite = msg
                    .channel_id
                    .create_invite(&ctx.http, |i| i.max_uses(0))
                    .await;
                if invite.is_ok() {
                    dm_announcement.push(invite.unwrap().url());
                }
                let mut guild_announcement = MessageBuilder::new();
                guild_announcement.push_line(&notice);
                guild_announcement.push_line(participants_text);
                guild_announcement
                    .push_line("TODO - notify of player removals from other game_modes")
                    .push_line("TODO - mount auto captain timer countdown");
                msg.channel_id.say(&ctx.http, guild_announcement).await?;
                for player in existing_players.iter() {
                    player
                        .get_user()
                        .direct_message(&ctx, |m| m.content(&dm_announcement))
                        .await?;
                }
                player_copy = Some(existing_players.clone());
            }

            // TODO: Notify all removed players
            let mut _removals: HashMap<&GameMode, &UserId> = HashMap::default();

            // then loop through all game modes:
            // - players that are in the filled pug are removed from all other game modes
            // - if currently evaluating the filled game mode, move all players to a PickingSession
            for (current_game_mode, participants) in pugs_waiting_to_fill_in_guild.iter_mut() {
                if current_game_mode == filled_game_mode {
                    let mut filled_pugs = lock_for_filled_pugs.write().await;

                    if let Some(filled_pugs_in_guild) = filled_pugs.get_mut(&guild_id) {
                        let mut picking_session =
                            PickingSession::new(&current_game_mode, participants.clone());
                        let mut session_is_ok_to_store = true;
                        // Special case: 2 player game mode
                        // Picking will complete automatically
                        // set_captain calls pick, which assigns this user to random team,
                        // then because there's only one user left, they get auto assigned
                        if filled_game_mode.capacity() == 2 {
                            // DO NOT  return OK(()) within this block, so as to keep the workflow consistent
                            // i.e. This game mode filled, so regardless of whether auto picking succeeded/failed,
                            // proceed to remove participant from any other pugs they're in
                            match picking_session.set_captain(msg.author.id) {
                                Ok(success) => {
                                    let response = match success {
                                        SetCaptainSuccess::TwoPlayerAutoPick {
                                            blue_captain,
                                            red_captain,
                                        } => MessageBuilder::new()
                                            .push_line("Teams have been auto selected:")
                                            .push_line(format!(
                                                "**Blue:** {}",
                                                blue_captain.mention()
                                            ))
                                            .push_line(format!(
                                                "**Red:** {}",
                                                red_captain.mention()
                                            ))
                                            .build(),
                                        _ => {
                                            session_is_ok_to_store = false;
                                            "Since there are only two players in this game mode,\
                                            I tried to auto assign teams but failed to complete.\
                                            As a result, this pug has been discarded."
                                                .to_string()
                                        }
                                    };

                                    msg.channel_id.say(&ctx.http, response).await?;
                                }
                                Err(_) => {
                                    let mut response = MessageBuilder::new();
                                    response.push_line("Oh no :(")
                                            .push("Since there are only two players involved,\
                                            I tried auto assigning you both to teams and something went wrong");
                                    msg.reply(&ctx.http, response).await?;
                                }
                            }
                        }
                        if session_is_ok_to_store {
                            filled_pugs_in_guild.push_back(picking_session);
                        }
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
                                _removals.insert(current_game_mode, &player.get_user().id);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
