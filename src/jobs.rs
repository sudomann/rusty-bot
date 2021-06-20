use crate::utils::stale_join;
use serenity::client::Context;
use std::sync::atomic::{
    AtomicBool,
    Ordering::{AcqRel, Acquire, SeqCst},
};
use tokio::time;
use tracing::info;

static JOBS_THREAD_INITIALIZED: AtomicBool = AtomicBool::new(false);
const FIVE_MINUTES: u64 = 300;
const ONE_MINUTE: u64 = 60;

pub fn start_jobs(ctx: Context) {
    info!("Launching task threads");
    if JOBS_THREAD_INITIALIZED
        .compare_exchange(false, true, AcqRel, Acquire)
        .is_ok()
    {
        JOBS_THREAD_INITIALIZED.store(true, SeqCst);
        tokio::spawn(async move {
            let mut interval = time::interval(time::Duration::from_secs(FIVE_MINUTES));
            loop {
                stale_join::remove_expired_players(&ctx).await;
                interval.tick().await;
            }
        });

        tokio::spawn(async move {
            let mut interval = time::interval(time::Duration::from_secs(ONE_MINUTE));
            loop {
                //db_jobs::backup_data(&ctx).await;
                // this job should use the db client, which MUST be behind
                // a mutex
                interval.tick().await;
            }
        });
    };
}

/*
TODO:
when db backup job runs, makes sure all the various collections of in-memory
storages stay around a given length. i.e. after backing up, trim as neccessary, discarding the oldest entries

When retrieving multiple documents, order by a date field which should indicate the order in which the documents where created
*/

/*

    Anything with PickingSession needs to be considerately stored
    and retrieved



    DesignatedPugChannel:
    -   on create, add to db first, then memory
    -   on delete, remove from db first, then remove from mem

    RegisteredGameModes:
    -   on create, add to db first, then memory
    -   on delete, remove from db first, then remove from mem

    PugsWaitingToFill:
    -   on join, add to memory
    -   on leave, remove from memory

    FilledPug, CompletedPug:
    -   .reset alters mem only
    -   .pick alters mem only
    -   .leave alters mem only
    -   .quit command, SIGTERM, SIGINT attempt to add to db before shutdown


    DefaultVoiceChannels:
    -   on create, add to db first, then memory
    -   on delete, remove from db first, then remove from mem

    * On cache_ready, load all from db into memory:
    -   DesignatedPugChannel
    -   RegisteredGameModes
    -   PugsWaitingToFill
    -   DefaultVoiceChannels

    * On cache_ready, load last 5 for each guild:
    -   PugsWaitingToFill
    -   FilledPug
            - if one or both captain captain spot is unfilled,
            send a message to start timer)
    -   CompletedPug


    * DB storage job every 1 min:
    -   PugsWaitingToFill
    -   FilledPug
    -   CompletedPug
    -   Only store if item type has more than 5 (save as const) elements currently in memory

    * DB storage on .quit command, SIGTERM, SIGINT signals:
    -   PugsWaitingToFill
    -   FilledPug
    -   CompletedPug


    * Every 5 mins, check for players that have been in pug for over 5 hours

*/
