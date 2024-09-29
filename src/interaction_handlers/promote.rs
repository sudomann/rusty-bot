use serenity::{client::Context, model::application::CommandInteraction};

pub async fn promote(_ctx: &Context, _interaction: &CommandInteraction) -> anyhow::Result<String> {
    Ok("Promotion".to_string())
}
