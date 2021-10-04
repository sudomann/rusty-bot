use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serenity::async_trait;
use serenity::builder::{CreateApplicationCommand, CreateApplicationCommands};
use serenity::model::channel::Message;
use serenity::model::gateway::{Activity, Ready};
use serenity::model::id::GuildId;
use serenity::model::interactions::application_command::{
    ApplicationCommand, ApplicationCommandInteractionDataOptionValue,
};
use serenity::model::interactions::{Interaction, InteractionResponseType};
use serenity::prelude::*;
use tracing::{error, info, instrument};

use crate::db::{DEFAULT_DB_NAME, DEFAULT_MONGO_READY_MAX_WAIT};
use crate::interaction_handlers::*;
use crate::jobs::{clear_out_stale_joins, log_system_load};
use crate::utils::onboarding::ensure_guild_registration;
use crate::{DbClientSetupHandle, DbRef};

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
            let content = match command.data.name.as_str() {
                "ping" => "Pong!".to_string(),
                "coinflip" => coin_flip::coin_flip(),
                "setpugchannel" => pug_channel::set(&ctx, &command).await,
                "id" => {
                    let options = command
                        .data
                        .options
                        .get(0)
                        .expect("Expected user option")
                        .resolved
                        .as_ref()
                        .expect("Expected user object");

                    if let ApplicationCommandInteractionDataOptionValue::User(user, _member) =
                        options
                    {
                        format!("{}'s id is {}", user.tag(), user.id)
                    } else {
                        "Please provide a valid user".to_string()
                    }
                }
                _ => "not implemented :(".to_string(),
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
        let empty: Vec<CreateApplicationCommand> = Vec::default();
        ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
            CreateApplicationCommands::set_application_commands(commands, empty)
        })
        .await
        .expect("expected successful deletion of all global commands");
    }

    // We use the cache_ready event just in case some cache operation is required in whatever use
    // case you have for this.
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
            // tokio::spawn creates a new green thread that can run in parallel with the rest of
            // the application.
            tokio::spawn(async move {
                loop {
                    // We clone Context again here, because Arc is owned, so it moves to the
                    // new function.
                    log_system_load(Arc::clone(&ctx1)).await;
                    tokio::time::sleep(Duration::from_secs(120)).await;
                }
            });

            // And of course, we can run more than one thread at different timings.
            let ctx2 = Arc::clone(&ctx);
            tokio::spawn(async move {
                loop {
                    clear_out_stale_joins(Arc::clone(&ctx2)).await;
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            });

            // Now that the loop is running, we set the bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
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
                info!("MongoDB client constructed successfully");
                // Get a handle to a database.
                let db = client.database(DEFAULT_DB_NAME);
                let mut data = ctx.data.write().await;
                data.insert::<DbRef>(db);
            }
            Err(err) => {
                if err.is_panic() {
                    // TODO: does this actually halt the bot during stack unwinding?
                    // Resume the panic on the main task
                    // panic::resume_unwind(err.into_panic());
                    err.into_panic();
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

        if let Err(err) = ensure_guild_registration(ctx, guilds).await {
            error!(
                "Error occured when checking guild registrations:\n{:#}",
                err
            );
        }
    }
}
