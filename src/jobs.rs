/*
use crate::{ban::unban_users, command_history::clear_command_history, SendSyncError, HOUR};
use serenity::client::Context;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::sleep,
    time::Duration,
};

static JOBS_THREAD_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub(crate) fn start_jobs(cx: Context) {
    if !JOBS_THREAD_INITIALIZED.load(Ordering::SeqCst) {
        JOBS_THREAD_INITIALIZED.store(true, Ordering::SeqCst);
        std::thread::spawn(move || -> Result<(), SendSyncError> {
            loop {
                unban_users(&cx)?;
                clear_command_history(&cx)?;

                sleep(Duration::new(HOUR, 0));
            }
        });
    }
}
*/

/*
TODO:
when db backup job runs, makes sure all the various collections of in-memory
storages stay around a given length. i.e. after backing up, trim as neccessary, discarding the oldest entries
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
    -   on cache_ready(), load all from db into memory

    * On cache_ready, load all from db into memory:
    -   DesignatedPugChannel
    -   RegisteredGameModes
    -   PugsWaitingToFill


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
