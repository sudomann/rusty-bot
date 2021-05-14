use serenity::model::id::ChannelId;

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

#[derive(Clone, Copy, Debug)]
pub struct TeamVoiceChannels {
    blue: Option<ChannelId>,
    red: Option<ChannelId>,
}

impl TeamVoiceChannels {
    pub fn new(blue: Option<ChannelId>, red: Option<ChannelId>) -> Self {
        Self { blue, red }
    }
    pub fn get_blue(&self) -> &Option<ChannelId> {
        &self.blue
    }
    pub fn get_red(&self) -> &Option<ChannelId> {
        &self.red
    }
    pub fn set_blue(&mut self, new_value: Option<ChannelId>) {
        self.blue = new_value;
    }
    pub fn set_red(&mut self, new_value: Option<ChannelId>) {
        self.red = new_value;
    }
}
