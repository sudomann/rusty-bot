use serenity::builder::CreateApplicationCommand;
use serenity::builder::CreateApplicationCommandOption;
use serenity::model::interactions::application_command::ApplicationCommandOptionType;
use serenity::model::prelude::User;

// -----------------
// Base command set
// -----------------

// FIXME: do these really need their own submodule?
// interaction_handlers::setup module determines which of these are called
pub mod base {
    use serenity::builder::{CreateApplicationCommand, CreateApplicationCommandOption};
    use serenity::model::interactions::application_command::ApplicationCommandOptionType;

    use crate::db::model::GameMode;

    pub fn build_help() -> CreateApplicationCommand {
        let mut cmd = CreateApplicationCommand::default();
        cmd.name("help").description("Show the manual for this bot");
        cmd
    }

    pub fn build_pugchannel() -> CreateApplicationCommand {
        let mut channel_option = CreateApplicationCommandOption::default();
        channel_option
            .name("channel")
            .description("Choose a text channel")
            .kind(ApplicationCommandOptionType::Channel)
            .required(true);

        let mut cmd = CreateApplicationCommand::default();
        cmd.name("setpugchannel")
            .description("Designate a channel to be used for pugs")
            .add_option(channel_option);

        cmd
    }

    pub fn build_addmod() -> CreateApplicationCommand {
        let mut label_option = CreateApplicationCommandOption::default();
        label_option
            .name("label")
            .description("Name of the game mode")
            .kind(ApplicationCommandOptionType::String)
            .required(true);

        let mut player_count_option = CreateApplicationCommandOption::default();
        player_count_option
        .name("player_count")
        .description(
            "Number of players required to fill the game mode. Must be even, minimum 2, maximum 24",
        )
        .kind(ApplicationCommandOptionType::Integer)
        .add_int_choice("2", 2)
        .add_int_choice("4", 4)
        .add_int_choice("6", 6)
        .add_int_choice("8", 8)
        .add_int_choice("10", 10)
        .add_int_choice("12", 12)
        .add_int_choice("14", 14)
        .add_int_choice("16", 16)
        .add_int_choice("18", 18)
        .add_int_choice("20", 20)
        .add_int_choice("22", 22)
        .add_int_choice("24", 24)
        .required(true);

        let mut cmd = CreateApplicationCommand::default();
        cmd.name("addmod")
            .description("Add a new game mode")
            .add_option(label_option)
            .add_option(player_count_option);

        cmd
    }

    pub fn build_delmod(game_modes: &Vec<GameMode>) -> CreateApplicationCommand {
        let mut game_mode_option = CreateApplicationCommandOption::default();

        // load existing game modes
        for existing_game_mode in game_modes {
            game_mode_option
                .add_string_choice(&existing_game_mode.label, &existing_game_mode.label);
        }

        game_mode_option
            .name("game_mode")
            .description("The label of the game mode you want to delete")
            .kind(ApplicationCommandOptionType::String)
            .required(true);

        let mut cmd = CreateApplicationCommand::default();
        cmd.name("delmod")
            .description("Delete an existing game mode")
            .add_option(game_mode_option);
        cmd
    }

    pub fn build_list() -> CreateApplicationCommand {
        let mut cmd = CreateApplicationCommand::default();
        cmd.name("list")
            .description("Show available game modes and queued players");
        cmd
    }

    pub fn build_last(game_modes: &Vec<GameMode>) -> CreateApplicationCommand {
        let mut history_count_option = CreateApplicationCommandOption::default();
        history_count_option
      .name("match_age")
      .description(
          "How many steps/matches to traverse into match history when searching for a match to display",
      )
      .kind(ApplicationCommandOptionType::Integer);

        let game_mode_option = generate_command_option_game_mode(&game_modes, false);

        let mut cmd = CreateApplicationCommand::default();
        cmd.name("last")
            .description("Display info about a previous pug. You can filter results by game mode.")
            .add_option(history_count_option)
            .add_option(game_mode_option);
        cmd
    }

    /// The join command only has one option, which is required.
    ///
    /// The choices are game mode labels that are obtained from the [`Vec<GameMode>`] provided
    /// to this function. No choices are added to the option if the [`Vec`] is empty.
    pub fn build_join(game_modes: &Vec<GameMode>) -> CreateApplicationCommand {
        // !FIXME: The planned behavior for join is for the game mode option to be optional,
        // so that when it is ommited, the user is added to all available game modes
        // Therefore, join handler should be updated accordingly. and `false` passed to the following function
        let game_mode_option = generate_command_option_game_mode(&game_modes, false);
        let mut cmd = CreateApplicationCommand::default();
        cmd.name("join")
            .description("Add yourself to all game mode queues, or one you specify")
            .add_option(game_mode_option);
        cmd
    }

    /// The leave command only has one option, which is required.
    ///
    /// The choices are game mode labels that are obtained from the [`Vec<GameMode>`] provided
    /// to this function. No choices are added to the option if the [`Vec`] is empty.
    pub fn build_leave(game_modes: &Vec<GameMode>) -> CreateApplicationCommand {
        let game_mode_option = generate_command_option_game_mode(&game_modes, false);
        let mut cmd = CreateApplicationCommand::default();
        cmd.name("leave")
            .description("Remove yourself from all game mode queues, or one you specify")
            .add_option(game_mode_option);
        cmd
    }

    pub fn build_addplayer(game_modes: &Vec<GameMode>) -> CreateApplicationCommand {
        let mut user_option = CreateApplicationCommandOption::default();
        let mut game_mode_option = CreateApplicationCommandOption::default();

        user_option
            .name("user")
            .description("Which user to add")
            .kind(ApplicationCommandOptionType::User)
            .required(true);

        // load existing game modes
        for existing_game_mode in game_modes {
            game_mode_option
                .add_string_choice(&existing_game_mode.label, &existing_game_mode.label);
        }

        game_mode_option
            .name("game_mode")
            .description("Which game mode queue you want to add the user to")
            .kind(ApplicationCommandOptionType::String)
            .required(true);

        let mut cmd = CreateApplicationCommand::default();
        cmd.name("addplayer")
            .description("Add a user to the queue for a game mode")
            .add_option(user_option)
            .add_option(game_mode_option);
        cmd
    }

    pub fn build_delplayer(game_modes: &Vec<GameMode>) -> CreateApplicationCommand {
        let mut user_option = CreateApplicationCommandOption::default();
        let mut game_mode_option = CreateApplicationCommandOption::default();

        user_option
            .name("user")
            .description("Which user to remove")
            .kind(ApplicationCommandOptionType::User)
            .required(true);

        // load existing game modes
        for existing_game_mode in game_modes {
            game_mode_option
                .add_string_choice(&existing_game_mode.label, &existing_game_mode.label);
        }

        game_mode_option
            .name("game_mode")
            .description("Which game mode queue you want to remove the user from")
            .kind(ApplicationCommandOptionType::String)
            .required(true);

        let mut cmd = CreateApplicationCommand::default();
        cmd.name("delplayer")
            .description("Remove a user from the queue of a game mode")
            .add_option(user_option)
            .add_option(game_mode_option);
        cmd
    }

    /// The join command only has one option, which is required.
    /// This helper builds that option.
    ///
    /// The choices are game mode labels that are obtained from the [`Vec<GameMode>`] provided
    /// to this function. No choices are added to the option if the [`Vec`] is empty.
    ///
    /// This functionality lives in a separate helper function so it might be reusable.
    // !TODO: since this is apparently no longer useful outside this module. should it be deleted?
    pub fn generate_command_option_game_mode(
        game_modes: &Vec<GameMode>,
        is_value_required: bool,
    ) -> CreateApplicationCommandOption {
        let mut game_mode_option = CreateApplicationCommandOption::default();
        game_mode_option
            .name("game_mode")
            .description("You can type-to-search for more if you don't see all choices")
            .kind(ApplicationCommandOptionType::String)
            .required(is_value_required);
        for game_mode in game_modes {
            // !TODO: verify that having no game modes, and thus no choices to add, does not result
            // in arbitrary strings allowed as input
            game_mode_option.add_string_choice(&game_mode.label, &game_mode.label);
        }
        game_mode_option
    }
}

// -----------------
// Picking session command set
// -----------------

pub fn build_captain() -> CreateApplicationCommand {
    let mut cmd = CreateApplicationCommand::default();
    cmd.name("captain")
        .description("Assume captain title in a filled pug");
    cmd
}

pub fn build_autocaptain() -> CreateApplicationCommand {
    let mut cmd = CreateApplicationCommand::default();
    cmd.name("autocaptain")
        .description("Coerce random captains for any available captain spots");
    cmd
}

pub fn build_nocaptain() -> CreateApplicationCommand {
    let mut cmd = CreateApplicationCommand::default();
    cmd.name("nocaptain")
        .description("Exclude yourself from random captain selection");
    cmd
}

/// Create a /pick command using a player list to create options.
pub fn build_pick(players: &Vec<User>) -> CreateApplicationCommand {
    let mut player_option = CreateApplicationCommandOption::default();

    player_option
        .name("player")
        .description("A user you want to pick for your team")
        .kind(ApplicationCommandOptionType::String)
        .required(true);

    for player in players {
        player_option.add_string_choice(&player.name, &player.id.0);
    }

    let mut cmd = CreateApplicationCommand::default();
    cmd.name("pick")
        .description("Choose a player for your team")
        .add_option(player_option);

    cmd
}

pub fn build_reset() -> CreateApplicationCommand {
    let mut cmd = CreateApplicationCommand::default();
    cmd.name("reset")
        .description("Reset a pug to be as if it just filled");
    cmd
}
