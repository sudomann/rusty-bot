use crate::{pug::game_mode::GameMode, RegisteredGameModes};
use itertools::join;
use serenity::{
    client::Context, framework::standard::Args, model::id::GuildId, utils::MessageBuilder,
};
use std::collections::HashSet;

/// GameModes which are parsed from string arguments submitted to a command.
/// The string arguments are matched against the registered game modes (if any) of the guild
/// the command was sent from and returned as respective [`GameMode`]s in a [`HashSet`].
pub type ParsedGameModes = HashSet<GameMode>;
pub enum GameModeError {
    NoneGiven(String),
    NoneRegistered(String),
    Foreign(String),
}

/// TODO: callers such as commands are responsible for having the right checks to make sure
/// the message came through the gateway and contains a GuildId, which they provide to this function
pub async fn parse_game_modes(
    ctx: &Context,
    guild_id: &GuildId,
    mut args: Args,
) -> Result<ParsedGameModes, GameModeError> {
    args.trimmed().quoted();
    if args.is_empty() {
        return Err(GameModeError::NoneGiven(
            "No game modes provided".to_string(),
        ));
    }

    let data = ctx.data.read().await;
    let lock_for_registered_game_modes = data
        .get::<RegisteredGameModes>()
        .expect("Expected RegisteredGameModes in TypeMap");
    let global_game_modes = lock_for_registered_game_modes.read().await;

    let guild_game_modes = global_game_modes.get(guild_id);

    let registered_game_modes = guild_game_modes.unwrap();
    if registered_game_modes.is_empty() {
        return Err(GameModeError::NoneRegistered(
            "No game modes registered. Contact admins to run `.addmod`".to_string(),
        ));
    }
    let game_mode_keys = registered_game_modes
        .iter()
        .map(|game_mode| game_mode.to_string())
        .collect::<HashSet<String>>();

    let command_args = args
        .iter::<String>()
        .filter(|arg| arg.is_ok())
        .map(|arg| arg.unwrap().to_lowercase())
        // TODO: fix borrow/comparison impls on GameMode so we don't have to keep lowercasing
        .collect::<HashSet<String>>();

    // get the values that are in 'command_args' but not in 'game_mode_keys'
    let unrecogized_game_modes = command_args
        .difference(&game_mode_keys)
        .map(|game_mode| game_mode.clone())
        .collect::<HashSet<String>>();

    if unrecogized_game_modes.is_empty() {
        let mut recognized_game_modes: ParsedGameModes = ParsedGameModes::default();
        for game_mode in registered_game_modes.iter() {
            if command_args.contains(game_mode.key()) {
                recognized_game_modes.insert(game_mode.clone());
            }
        }
        return Ok(recognized_game_modes);
    } else {
        let unrecognized_pretty_printed = join(unrecogized_game_modes, " :small_orange_diamond: ");
        let game_modes_pretty_printed = join(game_mode_keys, " :small_orange_diamond: ");
        let response = MessageBuilder::new()
            .push_line("Ignored")
            .push_line("Please double check the unknown game mode(s) you submitted:")
            .push_bold_line(unrecognized_pretty_printed)
            .push_line("Allowed game mode(s):")
            .push_bold(game_modes_pretty_printed)
            .build();
        return Err(GameModeError::Foreign(response));
    }
}
