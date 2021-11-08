use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Team {
    Blue,
    Red,
}

// These handlers use the interaction's source channel id to validate whether it is a pug channel/thread,
// then checks/validates the user (e.g. is part of that pug) before going into effect

pub async fn captain() {}

// This command updates `/pick` command options.
pub async fn pick() {}

pub async fn reset() {}
