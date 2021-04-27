use std::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
};

#[derive(Eq, Debug, Clone)]
pub struct GameMode {
    key: String,
    label: String,
    pub player_count: u8, // must be even
}

impl Hash for GameMode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl PartialEq<GameMode> for GameMode {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl PartialEq<String> for GameMode {
    fn eq(&self, other: &String) -> bool {
        self.key == other.to_lowercase()
    }
}

impl fmt::Display for GameMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key)
    }
}

impl Borrow<String> for GameMode {
    fn borrow(&self) -> &String {
        &self.key
    }
}

impl GameMode {
    pub fn new(label: String, player_count: u8) -> Self {
        GameMode {
            key: label.to_lowercase(),
            label,
            player_count,
        }
    }

    pub fn label(&self) -> &String {
        &self.label
    }

    pub fn key(&self) -> &String {
        &self.key
    }

    pub fn capacity(&self) -> u8 {
        self.player_count
    }
}
