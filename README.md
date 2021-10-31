# Rusty Bot

Discord bot written in **Rust**. Currently being developed specifically for Unreal Carnage discord server. If demand exists and it's feasible, support for other hubs can be taken into consideration.


# Get Started

TODO: general info

## Want to run this bot?

TODO: docker image build notes

## Extra

Pull requests welcome for bugs, increased code quality/performance!
Feature requests, or alteration of bot/command structure must come through discord, and have sufficient community/admin desire.


# Commands

on launch:

loop through guilds:
- do a check for presence of application commands by this bot
- if none exist, add /setup command


/setup command:
- delete all commands (if mismatch) and create command set (save their ids in db)
- command set is: /pugchannel, /addmod, /delmod, /last

/addmod:
if this is the first/only gamemode, also create/update /join, /leave, /leaveall, /list, /listall, /captain, /nocaptain, /reset, /forcerandomcaptain, /addplayer, /delplayer, /promote, /pick, /voices

/delmod:
after running, if no more game modes, remove/update all ^supplementary commands that /addmod created