use serenity::builder::CreateCommand;
use serenity::builder::CreateCommandOption;
use serenity::model::application::CommandOptionType;
use serenity::model::prelude::User;

pub mod name {
    const MY_GLOBAL_STRING: &str = "My global constant string";
}

// -----------------
// Base command set
// -----------------

// FIXME: do these really need their own submodule?
// interaction_handlers::setup module determines which of these are called
pub mod base {
    use serenity::builder::{CreateCommand, CreateCommandOption};
    use serenity::model::application::CommandOptionType;

    use crate::db::model::GameMode;

    pub fn build_help() -> CreateCommand {
        CreateCommand::new("help").description("Show the manual for this bot")
    }

    pub fn build_pugchannel() -> CreateCommand {
        let channel_option = CreateCommandOption::new(
            CommandOptionType::Channel,
            "channel",
            "Choose a text channel",
        )
        .required(true);

        CreateCommand::new("setpugchannel")
            .description("Designate a channel to be used for pugs")
            .add_option(channel_option)
    }

    pub fn build_addmod() -> CreateCommand {
        let label_option =
            CreateCommandOption::new(CommandOptionType::String, "label", "Name of the game mode")
                .required(true);

        let player_count_option = CreateCommandOption::new(
            CommandOptionType::Integer,
            "player_count",
            "Number of players required to fill the game mode. Must be even, minimum 2, maximum 24",
        )
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

        CreateCommand::new("addmod")
            .description("Add a new game mode")
            .add_option(label_option)
            .add_option(player_count_option)
    }

    pub fn build_delmod(game_modes: &Vec<GameMode>) -> CreateCommand {
        let mut game_mode_option = CreateCommandOption::new(
            CommandOptionType::String,
            "game_mode",
            "The label of the game mode you want to delete",
        )
        .required(true);

        // load existing game modes
        for existing_game_mode in game_modes {
            game_mode_option = game_mode_option
                .add_string_choice(&existing_game_mode.label, &existing_game_mode.label);
        }

        CreateCommand::new("delmod")
            .description("Delete an existing game mode")
            .add_option(game_mode_option)
    }

    pub fn build_list() -> CreateCommand {
        CreateCommand::new("list").description("Show available game modes and queued players")
    }

    pub fn build_last(game_modes: &Vec<GameMode>) -> CreateCommand {
        let history_count_option = CreateCommandOption::new(CommandOptionType::Integer, "match_age", "How many steps/matches to traverse into match history when searching for a match to display");

        let game_mode_option = generate_command_option_game_mode(&game_modes, false);

        CreateCommand::new("last")
            .description("Display info about a previous pug. You can filter results by game mode.")
            .add_option(history_count_option)
            .add_option(game_mode_option)
    }

    /// The join command only has one option, which is required.
    ///
    /// The choices are game mode labels that are obtained from the [`Vec<GameMode>`] provided
    /// to this function. No choices are added to the option if the [`Vec`] is empty.
    pub fn build_join(game_modes: &Vec<GameMode>) -> CreateCommand {
        // !FIXME: The planned behavior for join is for the game mode option to be optional,
        // so that when it is ommited, the user is added to all available game modes
        // Therefore, join handler should be updated accordingly. and `false` passed to the following function
        let game_mode_option = generate_command_option_game_mode(&game_modes, false);
        CreateCommand::new("join")
            .description("Add yourself to all game mode queues, or one you specify")
            .add_option(game_mode_option)
    }

    /// The leave command only has one option, which is required.
    ///
    /// The choices are game mode labels that are obtained from the [`Vec<GameMode>`] provided
    /// to this function. No choices are added to the option if the [`Vec`] is empty.
    pub fn build_leave(game_modes: &Vec<GameMode>) -> CreateCommand {
        let game_mode_option = generate_command_option_game_mode(&game_modes, false);
        CreateCommand::new("leave")
            .description("Remove yourself from all game mode queues, or one you specify")
            .add_option(game_mode_option)
    }

    pub fn build_addplayer(game_modes: &Vec<GameMode>) -> CreateCommand {
        let user_option =
            CreateCommandOption::new(CommandOptionType::User, "user", "Which user to add")
                .required(true);
        let mut game_mode_option = CreateCommandOption::new(
            CommandOptionType::String,
            "game_mode",
            "Which game mode queue you want to add the user to",
        )
        .required(true);

        // load existing game modes
        for existing_game_mode in game_modes {
            game_mode_option = game_mode_option
                .add_string_choice(&existing_game_mode.label, &existing_game_mode.label);
        }

        CreateCommand::new("addplayer")
            .description("Add a user to the queue for a game mode")
            .add_option(user_option)
            .add_option(game_mode_option)
    }

    pub fn build_delplayer(game_modes: &Vec<GameMode>) -> CreateCommand {
        let user_option =
            CreateCommandOption::new(CommandOptionType::User, "user", "Which user to remove")
                .required(true);
        let mut game_mode_option = CreateCommandOption::new(
            CommandOptionType::String,
            "game_mode",
            "Which game mode queue you want to remove the user from",
        )
        .required(true);

        // load existing game modes
        for existing_game_mode in game_modes {
            game_mode_option = game_mode_option
                .add_string_choice(&existing_game_mode.label, &existing_game_mode.label);
        }

        CreateCommand::new("delplayer")
            .description("Remove a user from the queue of a game mode")
            .add_option(user_option)
            .add_option(game_mode_option)
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
    ) -> CreateCommandOption {
        let mut game_mode_option = CreateCommandOption::new(
            CommandOptionType::String,
            "game_mode",
            "You can type-to-search for more if you don't see all choices",
        )
        .required(is_value_required);
        for game_mode in game_modes {
            // !TODO: verify that having no game modes, and thus no choices to add, does not result
            // in arbitrary strings allowed as input
            game_mode_option =
                game_mode_option.add_string_choice(&game_mode.label, &game_mode.label);
        }
        game_mode_option
    }
}

// -----------------
// Picking session command set
// -----------------

pub fn build_captain() -> CreateCommand {
    CreateCommand::new("captain").description("Assume captain title in a filled pug")
}

pub fn build_autocaptain() -> CreateCommand {
    CreateCommand::new("autocaptain")
        .description("Coerce random captains for any available captain spots")
}

pub fn build_nocaptain() -> CreateCommand {
    CreateCommand::new("nocaptain").description("Exclude yourself from random captain selection")
}

/// Create a /pick command using a player list to create options.
pub fn build_pick(players: &Vec<User>) -> CreateCommand {
    let mut player_option = CreateCommandOption::new(
        CommandOptionType::String,
        "player",
        "A user you want to pick for your team",
    )
    .required(true);

    for player in players {
        // Sending the number itself seems to not work, so converting to string
        player_option = player_option.add_string_choice(&player.name, player.id.get().to_string());
    }

    CreateCommand::new("pick")
        .description("Choose a player for your team")
        .add_option(player_option)
}

pub fn build_teams() -> CreateCommand {
    CreateCommand::new("teams").description("Show teams for the current pug")
}

pub fn build_reset() -> CreateCommand {
    CreateCommand::new("reset").description("Reset a pug to be as if it just filled")
}
