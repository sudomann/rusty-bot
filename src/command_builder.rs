// -----------------
// Base command set
// -----------------

// FIXME: do these really need their own submodule?
// interaction_handlers::setup module determines which of these are called
pub mod base {
    use mongodb::Database;
    use serenity::builder::{CreateApplicationCommand, CreateApplicationCommandOption};
    use serenity::model::interactions::application_command::ApplicationCommandOptionType;

    use crate::db::model::GameMode;

    pub async fn build_pugchannel(
        _db: Database,
    ) -> Result<CreateApplicationCommand, mongodb::error::Error> {
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

        Ok(cmd)
    }

    pub async fn build_addmod(
        _db: Database,
    ) -> Result<CreateApplicationCommand, mongodb::error::Error> {
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

        Ok(cmd)
    }

    pub async fn build_delmod(
        db: Database,
    ) -> Result<CreateApplicationCommand, mongodb::error::Error> {
        let mut game_mode_option = CreateApplicationCommandOption::default();

        // load existing game modes
        for existing_game_mode in crate::db::read::get_game_modes(db).await?.iter() {
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
        Ok(cmd)
    }

    pub async fn build_last(
        _db: Database,
    ) -> Result<CreateApplicationCommand, mongodb::error::Error> {
        let mut history_count_option = CreateApplicationCommandOption::default();
        history_count_option
      .name("match_age")
      .description(
          "How many steps/matches to traverse into match history when searching for a match to display",
      )
      .kind(ApplicationCommandOptionType::Integer);

        let mut cmd = CreateApplicationCommand::default();
        cmd.name("last")
            .description("Display info about a previous pug")
            .add_option(history_count_option);
        Ok(cmd)
    }

    /// When no `Vec<GameMode>` is provided, this function will fetch from the db
    pub async fn build_join(
        db: Database,
        maybe_game_modes: Option<Vec<GameMode>>,
    ) -> Result<CreateApplicationCommand, mongodb::error::Error> {
        let game_modes = match maybe_game_modes {
            Some(game_modes) => game_modes,
            None => crate::db::read::get_game_modes(db).await?,
        };
        let game_mode_option = generate_join_command_option(&game_modes).await?;
        let mut cmd = CreateApplicationCommand::default();
        cmd.name("join")
            .description("Join a pug")
            .add_option(game_mode_option);
        Ok(cmd)
    }

    /// The join command only has one option, which is also required.
    /// This helper builds that option.
    ///
    /// The choices are game mode labels that are obtained from the [`Vec<GameMode>`] provided
    /// to this function. No choices are added to the option if the [`Vec`] is empty.
    pub async fn generate_join_command_option(
        game_modes: &Vec<GameMode>,
    ) -> Result<CreateApplicationCommandOption, mongodb::error::Error> {
        let mut game_mode_option = CreateApplicationCommandOption::default();
        game_mode_option
            .name("game_mode")
            .description("You can type-to-search for more if you don't see all choices")
            .kind(ApplicationCommandOptionType::String)
            .required(true);
        for game_mode in game_modes {
            game_mode_option.add_string_choice(&game_mode.label, &game_mode.label);
        }
        Ok(game_mode_option)
    }
}
