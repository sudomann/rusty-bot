# Rusty Bot

Discord bot written in **Rust**. Currently being developed specifically for Unreal Carnage discord server. If demand exists and it's feasible, support for other hubs can be taken into consideration.

## What this Bot Does Once Launched

1. Loops through a list of all currently connected guilds:
   - Checks the database for any guild application commands created by this bot
   - if none exist, add /setup command
   - The only guild commands examined are those created by this bot.
  
2. Begins listening for commands


## Want to run this bot?

TODO: docker image build notes

## Commands

/help, .help, !help
Prints this info

.configure
Creates application commands for the guild, customizing/configuring them using guild data from the database if there is any. If a mismatch if found between the current application commands in the guild and the command records in the database, they are all wiped before this process is carried out

/pugchannel
Designate the current channel as a pug channel for all game modes

/addmod
Add a new game mode
/delmod
Delete an existing game mode. Blocks if the game mode's queue is not empty, or picking is in progress for this game mode.

/addplayer
Add a player to a game mode's queue
/delplayer
Remove a player from a game mode's queue

/list
Show available game modes and queued players

/join
"Add yourself to all game mode queues, or one you specify"
/leave
"Remove yourself from all game mode queues, or one you specify"

*The next four commands only exist during a picking session*
/captain
/autocaptain
/pick
/reset

/last
View info about previous pugs
e.g. `last [game_mode] [how_many_games_ago]`

/coinflip
Flip a coin for a 50/50 chance of getting either heads or tails

.ping
Basic liveness check for the bot

.configure
A hidden diagnostic command for privileged users to clear then recreate a guild's application commands


## Notes

Some commands have options which need to be updated in response to database records being modified by other commands.
/addmod requires the following commands to be updated with the new game mode:
- /join
- /leave
- /delmod
- /last
- /addplayer
- /delplayer

/delmod requires the following commands to have the deleted game mode removed from the list of options:
- /join
- /leave
- /delmod
- /last
- /addplayer
- /delplayer

Since /captain and /randomcaptain can result in the last captain slot getting filled up, they both have the capability to handle 2 player game modes such as *duel*, which do not have a picking process.


## Extra

Pull requests welcome for bug fixes and code quality/performance improvements!
Feature requests, or suggestions to alter functionality of command structure must come through discord, backed by sufficient community/admin desire.