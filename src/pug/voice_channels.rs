// This gets the latest item in CompletedPug list (if it exists) and
// attaches a VoiceChannels instance (if channel creation succeeds) to its `voice_channels` field
// then moves players

// it only works if the pugsession isnt old (define this "old" - maybe completed more than 5 mins ago)

// .voices command checks `voice_channels` field. If none, tries to create and produce invite
// if they are present already, produce invites
// if creation fails, send error msg

/* creating a channel:
https://discord.com/developers/docs/resources/guild#create-guild-channel-json-params






the parent_id field can be populated with value from desginated default voice channels'
category ids: https://docs.rs/serenity/0.10.5/serenity/model/channel/struct.GuildChannel.html#structfield.category_id


use serenity::model::ChannelType;
let _ = guild
    .create_channel(&http, |c| c.name("my-test-channel").kind(ChannelType::Voice).id(  see parent_id comment above ^))
    .await;
*/
