use std::sync::Arc;

use serenity::http::Http;
use serenity::model::id::GuildId;
use serenity::model::interactions::application_command::{
    ApplicationCommand, ApplicationCommandOptionType,
};
use serenity::prelude::SerenityError;

/// Utility for setting the salsh commands of a guild.
///
/// Uses other guild data such as registered game modes, to customize the options if possible.
pub async fn construct_guild_commands(
    http: Arc<Http>,
    guild_id: &GuildId,
) -> Result<Vec<ApplicationCommand>, SerenityError> {
    guild_id
        .set_application_commands(&http, |commands| {
            commands
              .create_application_command(|command| {
                command.name("ping").description("A ping command")
              })
              .create_application_command(|command| {
                  command.name("assemble").description("Move players you recently played a pug with into your current voice channel")
              })
              .create_application_command(|command| {
                  command
                      .name("captain")
                      .description("Assume captain title in a filled pug")
              })
              .create_application_command(|command| {
                  command
                      .name("coinflip")
                      .description("50/50 chance of heads or tails")
              })
              .create_application_command(|command| {
                  command.name("echo").description("What is this, Ben10?")
              })
              .create_application_command(|command| {
                  command
                      .name("setactivity")
                      .description("Set the bot's activity")
              })
              .create_application_command(|command| {
                  command.name("addmod").description("Add a new game mode")
              })
              .create_application_command(|command| {
                  command
                      .name("delmod")
                      .description("Delete an existing game mode")
              })
              .create_application_command(|command| {
                  command
                      .name("here")
                      .description("With this bot, you don't need this command")
              })
              .create_application_command(|command| {
                  command.name("join").description("Join a pug")
              })
              .create_application_command(|command| {
                  command
                      .name("last")
                      .description("Display info about a previous pug")
              })
              .create_application_command(|command| {
                  command.name("leave").description("Quit a specific pug")
              })
              .create_application_command(|command| {
                  command
                      .name("leaveall")
                      .description("Quit all pugs you joined")
              })
              .create_application_command(|command| {
                  command.name("nocapt").description(
                      "Exclude yourself from random captain selection in a pug that just filled",
                  )
              })
              .create_application_command(|command| {
                  command
                      .name("list")
                      .description("Display how many people have joined the available game modes")
              })
              .create_application_command(|command| {
                  command
                      .name("listall")
                      .description("Display who has joined the available game modes")
              })
              .create_application_command(|command| {
                  command.name("git").description(
                      "HBO Special - Know what that is? It's the `help a brother out` special",
                  )
              })
              .create_application_command(|command| {
                  command
                      .name("pick")
                      .description("Choose a player for your team")
              })
              .create_application_command(|command| {
                  command
                      .name("promote")
                      .description("Promote pugs with @here")
              })
              .create_application_command(|command| {
                  command
                      .name("setpugchannel")
                      .description("Designate a channel to be used for pugs")
              })
              .create_application_command(|command| {
                  command
                      .name("addplayer")
                      .description("Add a player to a pug")
              })
              .create_application_command(|command| {
                  command
                      .name("delplayer")
                      .description("Remove a player from a pug")
              })
              .create_application_command(|command| {
                  command
                      .name("reset")
                      .description("Reset a pug to be as if it just filled")
              })
              .create_application_command(|command| {
                  command
                      .name("teams")
                      .description("Display team composition for the most recent pug")
              })
        })
        .await
}
