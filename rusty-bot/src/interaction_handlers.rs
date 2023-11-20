pub mod configure;
pub mod gambling;
pub mod game_mode;
pub mod meta;
pub mod picking_session;
pub mod player;
pub mod promote;
pub mod pug_channel;
pub mod queue;

/// Simple enum to represent whether all game modes, or
/// a single, specific game mode should be operated upon.
pub enum IntendedGameMode {
    /// Specific game mode label
    Single(String),
    /// All available game modes
    All,
}
