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
use crate::jobs::{clear_out_stale_joins, log_system_load};
use crate::utils::onboarding::inspect_guild_commands;
use crate::{DbClientRef, DbClientSetupHandle};

#[derive(Debug)]
pub struct Handler {
    pub(crate) is_loop_running: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with(".ping") {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                eprintln!("Error sending message: {:?}", why);
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let handler_result: anyhow::Result<String> = match command.data.name.as_str() {
                "ping" => Ok("Pong!".to_string()),
                "coinflip" => Ok(coin_flip::coin_flip()),
                "setpugchannel" => pug_channel::set(&ctx, &command).await,
                "setup" => setup::set_guild_base_command_set(&ctx, &command).await,
                "addmod" => game_mode::create(&ctx, &command).await,
                "delmod" => game_mode::delete(&ctx, &command).await,
                "join" => queue::join(&ctx, &command).await,
                "leave" => queue::leave(&ctx, &command).await,
                "captain" => picking_session::captain(&ctx, &command).await,
                "randomcaptain" => picking_session::random_captains(&ctx, &command).await,
                "pick" => picking_session::pick(&ctx, &command).await,
                _ => Ok("Not usable. Sorry :(".to_string()),
            };

            let content = match handler_result {
                Ok(response) => response,
                Err(err) => {
                    let event_id = nanoid!(6);
                    error!("Error Event [{}]\n{:#?}", event_id, err);
                    format!(
                        "Sorry, something went wrong and this incident has been logged. Incident ID: {}",
                        event_id
                    )
                }
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                error!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
        ctx.set_activity(Activity::playing("Bugs? Message sudomann#9568"))
            .await;

        // WARNING: This was annoying to figure out
        // DO NOT DISCARD THE FOLLOWING
        // It is useful for cleaning up global commands
        // let empty: Vec<CreateApplicationCommand> = Vec::default();
        // ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
        //     CreateApplicationCommands::set_application_commands(commands, empty)
        // })
        // .await
        // .expect("expected successful deletion of all global commands");

        let db_client_handle = {
            let mut data = ctx.data.write().await;
            data.remove::<DbClientSetupHandle>()
                .expect("Expected DbClientSetupHandle in TypeMap")
        };
        // TODO: put a 30 second time-out on db setup wait
        //  let await_timeout =
        //      std::env::var("MONGO_READY_MAX_WAIT").unwrap_or(DEFAULT_MONGO_READY_MAX_WAIT);
        // tokio::time::timeout(Duration::from_secs(await_timeout), db_client_handle).await???;
        match db_client_handle.await {
            Ok(client) => {
                info!("The MongoDB client connection to the database deployment is live");
                let mut data = ctx.data.write().await;
                data.insert::<DbClientRef>(client);
            }
            Err(err) => {
                if err.is_panic() {
                    // TODO: does this actually halt the bot during stack unwinding?
                    // Resume the panic on the main task
                    // panic::resume_unwind(err.into_panic());
                    // err.into_panic();
                    error!(
                        "Failed to establish connection between client and database. Exiting..."
                    );
                    process::exit(1);
                } else {
                    // TODO: handle this case where joining thread failed for some reason other
                    // than db::setup() panicking
                    panic!(
                        "Failed to join the thread that was supposed to create the \
                    mongodb client object. Perhaps it was cancelled?"
                    );
                }
            }
        }
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
            data.get::<DbClientRef>().unwrap().clone()
        };

        let db = db_client.database(&guild.id.to_string());

        info!("Launching onboarding task (perform an inspection) for the new guild");

        tokio::spawn(crate::utils::onboarding::inspect_and_maybe_update_db(
            Arc::new(ctx),
            guild.id,
            db_client.clone(),
        ));
    }
}
