use serenity::utils::MessageBuilder;

pub fn render_help_text() -> String {
    let mut response = MessageBuilder::new();
    response
        .push_line("Unimplemented")
        .push_line("Server admins, use `.configure` to set up (or update) my slash commands.");
    response.to_string()
}
