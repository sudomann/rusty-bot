use std::panic;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use nanoid::nanoid;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::{Activity, Ready};
use serenity::model::guild::Guild;
use serenity::model::id::GuildId;
use serenity::model::interactions::{Interaction, InteractionResponseType};
use serenity::prelude::*;
use tracing::{error, info, instrument};

// use crate::db::DEFAULT_MONGO_READY_MAX_WAIT;
use crate::interaction_handlers::*;
use crate::jobs::{clear_out_stale_joins, log_system_load, remove_stale_team_voice_channels};
use crate::utils::onboarding::inspect_guild_commands;
use crate::DbClientRef;

#[derive(Debug)]
pub struct Handler {
    pub(crate) is_loop_running: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // TODO: clean this up, make it easier to follow
        let msg_content = msg.content.to_lowercase();
        let response = if msg_content.starts_with(".ping") {
            "Pong!".to_string()
        } else if msg_content.starts_with(".help") || msg_content.starts_with("!help") {
            meta::render_help_text()
        } else if msg_content.starts_with(".configure") {
            if msg.guild_id.is_some() {
                match configure::generate_and_apply_guild_command_set(&ctx, &msg).await {
                    Ok(x) => x,
                    Err(err) => {
                        let event_id = nanoid!(6);
                        error!("Error Event [{}]\n{:#?}", event_id, err);
                        format!(
                        "Sorry, something went wrong and this incident has been logged. Incident ID: {}",
                        event_id
                    )
                    }
                }
            } else {
                return;
            }
        } else {
            return;
        };
        if let Err(why) = msg.reply(&ctx.http, response).await {
            eprintln!("Error sending message: {:?}", why);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            info!("Interaction:\n{:?}", command);
            let _working = command.channel_id.start_typing(&ctx.http);
            // Send an immediate initial response, so that the interaction token
            // does not get get invalidated after the initial time limit of 3 secs.
            // This makes the token valid for 15 mins, allowing the command handlers more than enough time to respond
            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content("Working on it..."))
                })
                .await
            {
                error!("Cannot respond to slash command: {}", why);
                return;
            }
            let handler_result: anyhow::Result<String> =
                match command.data.name.as_str().to_lowercase().as_str() {
                    "ping" => Ok("Pong!".to_string()),
                    "help" => Ok(meta::render_help_text()),
                    "coinflip" => Ok(gambling::coin_flip()),
                    "setpugchannel" => pug_channel::set(&ctx, &command).await,
                    "addmod" => game_mode::create(&ctx, &command).await,
                    "delmod" => game_mode::delete(&ctx, &command).await,
                    "join" => queue::join(&ctx, &command).await,
                    "leave" => queue::leave(&ctx, &command).await,
                    "addplayer" => player::add_to_pug(&ctx, &command).await,
                    "delplayer" => player::remove_from_pug(&ctx, &command).await,
                    "list" => queue::list(&ctx, &command).await,
                    "captain" => picking_session::captain(&ctx, &command).await,
                    "autocaptain" => picking_session::auto_captain(&ctx, &command).await,
                    "pick" => picking_session::pick(&ctx, &command).await,
                    "reset" => picking_session::reset(&ctx, &command).await,
                    "last" => meta::pug_history(&ctx, &command).await,
                    _ => Ok("Not usable. Sorry :(".to_string()),
                };

            let actual_response = match handler_result {
                Ok(response) => response,
                Err(err) => {
                    let event_id = nanoid!(6);
                    error!("Error Event [{}]\n{:#?}", event_id, err);
                    format!(
                        "Sorry, something went wrong and this incident has been logged.\nIncident ID: `{}`",
                        event_id
                    )
                }
            };

            if let Err(why) = command
                .edit_original_interaction_response(&ctx.http, |initial_response| {
                    initial_response.content(actual_response)
                })
                .await
            {
                error!("Cannot update initial interaction response: {}", why);
            }
            _working
                .expect(
                    "Expected typing to have begun successfully - so that it could now be stopped",
                )
                .stop();
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
        ctx.set_activity(Activity::playing("Bugs? Message sudomann#9568"))
            .await;
    }

    #[instrument(skip(self, ctx, guilds))]
    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        info!("Cache built successfully!");

        // it's safe to clone Context, but Arc is cheaper for this use case.
        // Untested claim, just theoretically. :P
        let ctx = Arc::new(ctx);

        // We need to check that the loop is not already running when this event triggers,
        // as this event triggers every time the bot enters or leaves a guild, along every time the
        // ready shard event triggers.
        //
        // An AtomicBool is used because it doesn't require a mutable reference to be changed, as
        // we don't have one due to self being an immutable reference.
        if !self.is_loop_running.load(Ordering::Relaxed) {
            // We have to clone the Arc, as it gets moved into the new thread.
            let ctx1 = Arc::clone(&ctx);
            let ctx2 = Arc::clone(&ctx);
            let ctx3 = Arc::clone(&ctx);

            tokio::spawn(async move {
                loop {
                    // We clone Context again here, because Arc is owned, so it moves to the
                    // new function.
                    log_system_load(Arc::clone(&ctx1)).await;
                    tokio::time::sleep(Duration::from_secs(120)).await;
                }
            });

            tokio::spawn(async move {
                loop {
                    clear_out_stale_joins(Arc::clone(&ctx2)).await;
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            });

            let copy_of_guilds = guilds.clone();
            tokio::spawn(async move {
                loop {
                    remove_stale_team_voice_channels(&ctx3, &copy_of_guilds).await;
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
            });

            // Now that the loop is running, we set the bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }

        // TODO: maybe calling this with tokio in a blocking manner will enable guild_create handler to
        // work correctly (in the case that an absent Client causes guild_create handler to panic)
        inspect_guild_commands(ctx, guilds).await;
    }

    #[instrument(skip(self, ctx))]
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        // TODO: test addition of new new guild while inspect_guild_commands() is looping/waiting
        // for Client to exist. Hopefully, out of the box, this handler never fires until cache_ready completes
        // and a Client is guaranteed to be in storage

        if !is_new {
            return;
        }

        info!(
            "New guild (GuildId: {}) connected - {}",
            guild.id.0, guild.name
        );

        // do onboarding for guilds added after the bot was launched
        let db_client = {
            let data = ctx.data.read().await;

            // !FIXME: must block at laucnh while waiting for DB. Bot begins listening otherwise,
            // and will fail on commands that need the db client
            data.get::<DbClientRef>()
                .expect("Expected MongoDB's `Client` to be available for use")
                .clone()
        };

        info!("Launching onboarding task (perform an inspection) for the new guild");

        tokio::spawn(crate::utils::onboarding::inspect_and_maybe_update_db(
            Arc::new(ctx),
            guild.id,
            db_client.clone(),
        ));
    }
}
