use serenity::client::Context;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::utils::MessageBuilder;

pub fn render_help_text() -> String {
    let mut response = MessageBuilder::new();
    response
        .push_line("Type slash (`/`) and search available commands to see their description.")
        .push_line("Server admins, use `.configure` to set up (or update) my slash commands.");
    response.to_string()
}

pub async fn pug_history(
    _ctx: &Context,
    _interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    // validate this channel is a GuildChannel
    // with kind PublicThread

    // check for an active pug

    // Clear all captains and picks

    // Restart autocap timer

    Ok("Sorry, this feature is incomplete".to_string())
}
