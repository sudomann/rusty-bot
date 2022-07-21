use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
};

pub async fn promote(
    _ctx: &Context,
    _interaction: &ApplicationCommandInteraction,
) -> anyhow::Result<String> {
    Ok("Promotion".to_string())
}
