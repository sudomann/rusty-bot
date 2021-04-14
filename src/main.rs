mod commands;

use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::standard::{
        help_commands,
        macros::{check, group, help, hook},
        Args, CommandGroup, CommandOptions, CommandResult, DispatchError, HelpOptions, Reason,
        StandardFramework,
    },
    http::Http,
    model::{
        channel::{GuildChannel, Message},
        event::ResumedEvent,
        gateway::Ready,
        id::{GuildId, UserId},
    },
    prelude::*,
    utils::MessageBuilder,
};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use commands::{ join::*,  leave::*, list::*, meta::*,  owner::*, pick::*, promote::* };

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct DesignatedPugChannel;

impl TypeMapKey for DesignatedPugChannel {
    type Value = Arc<RwLock<HashMap<GuildId, GuildChannel>>>;
}

// TODO - represent pug queue similarly ^

struct Handler;
const DEFAULT_PUG_CHANNEL_NAME: &str = "pugs-test";

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }
    async fn cache_ready(&self, context: Context, guild_ids: Vec<GuildId>) {
        // load into global storage the designated pug channel for each guild
        let mut designated_pug_channels = HashMap::default();
        for guild_id in guild_ids.iter() {
            match context.cache.guild_channels(guild_id).await {
                Some(guild_channels) => {
                    for (_channel_id, channel) in guild_channels {
                        if channel.name == DEFAULT_PUG_CHANNEL_NAME {
                            designated_pug_channels.insert(*guild_id, channel);
                            // Current implementation assumes there is no more
                            // than one designated channel for pugging in a guild
                            // Thus we terminate this loop once we find it
                            break;
                        }
                    }
                }
                // TODO: report that somehow a guild returned ... no channels ???
                None => continue,
            };
        }

        {
            let mut data = context.data.write().await;
            data.insert::<DesignatedPugChannel>(Arc::new(RwLock::new(designated_pug_channels)));
        }
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[hook]
async fn dispatch_error_hook(context: &Context, msg: &Message, error: DispatchError) {
    // TODO: convert `if let` to `match` when handling the other errors
    // such as `DispatchError::LackOfPermissions`, etc.
    if let DispatchError::CheckFailed(_, reason) = error {
        match reason {
            Reason::User(info) => {
                msg.reply(&context.http, &info)
                    .await
                    .expect("Expected informational string about the failed check");
                return;
            }
            _ => panic!("Unimplemented response for CheckFailed event"),
        }
    }
}

#[check]
#[name = "PugChannel"]
async fn is_pug_channel_check(
    context: &Context,
    msg: &Message,
    _: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    if let Some(guild_id) = msg.guild_id {
        // get the channel this message came from
        let current_channel: GuildChannel = match msg.channel_id.to_channel(&context).await {
            Ok(channel) => channel.guild().unwrap(),
            // TODO: remove panic - this is probably be salvageable
            Err(_why) => panic!("Failed to determine channel of message"),
        };

        // TODO: try to narrow this body as much as possible
        // to reduce time spent holding the RWLock in read mode
        let fail: Option<String> = {
            let data_read = context.data.read().await;
            // Then we obtain the value we need from data, in this case, we want the desginated pug channels.
            // The returned value from get() is an Arc, so the reference will be cloned, rather than
            // the data.
            let pug_channels_lock = data_read
                .get::<DesignatedPugChannel>()
                .expect("Expected DesignatedPugChannel in TypeMap")
                .clone();
            let pug_channels = pug_channels_lock.read().await;

            // Then use the designated pug channel of the guild this message came from
            // This time, the value is not Arc, so the data will be cloned.
            match pug_channels.get(&guild_id) {
                Some(pug_channel_for_current_guild) => {
                    if current_channel.name != *pug_channel_for_current_guild.name {
                        Some(MessageBuilder::new()
                            .push("Please go to the ")
                            .mention(pug_channel_for_current_guild)
                            .push(" channel to use this command")
                            .build())
                    }
                    else {None} 
                },
                None => {
                    Some(MessageBuilder::new()
                    .push("No pug channel set. Contact admins to type `.setpugchannel` in the channel destined for pugs.")
                    .build())
                },
            }
        };
        if let Some(response) = fail {
            Err(Reason::User(response))
        }
        else{Ok(())}
        
    } else {
        panic!("No GuildId in received message - Is client running without gateway?");
    }
}

#[help]
#[individual_command_tip = "If you want more information about a specific command, just ..."]
#[command_not_found_text = "Could not find: `{}`."]
#[max_levenshtein_distance(3)]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[group]
#[commands(join, ping, leave, leave_all, list, list_all,  pick, promote, quit)]
#[checks(PugChannel)]
struct General;

#[tokio::main]
async fn main() {
    // Consult `./.env.example` for sample
    dotenv::dotenv().expect("Failed to load .env file");

    // Initialize the logger using environment variables.
    // TODO: get `RUST_LOG` off debug in prod.
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);

    // Fetch bot owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| {
            c.with_whitespace(true)
                .case_insensitivity(true)
                .owners(owners)
                .prefix(".")
        })
        .on_dispatch_error(dispatch_error_hook)
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();

    // TODO: attach perhaps a handler to announce to admins that bot is going offline
    // How to pass reason? Distinguish SIGINT and SIGTERM
    // Neglect Windows support :/
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
