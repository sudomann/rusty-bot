use chrono::{prelude::*, Duration};
use linked_hash_set::LinkedHashSet;
use serenity::model::{id::UserId, prelude::User};
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

pub type Players = LinkedHashSet<Player>;

#[derive(Eq, Debug, Clone)]
pub struct Player {
    // TODO: `join_datetime` field might interfer with comparison
    // consider manually implementing comparison of UserId's
    user: User,
    join_datetime: DateTime<Utc>,
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.user.id == other.user.id
    }
}

impl PartialEq<UserId> for Player {
    fn eq(&self, other: &UserId) -> bool {
        self.user.id == *other
        // how is this different from
        // &self.user_id == other
    }
}

impl Hash for Player {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.user.id.hash(state);
    }
}

impl Borrow<UserId> for Player {
    /// Facilitates identifying instances of [`PugParticipant`]
    /// within collections, so get, insertion, removal, can be done
    /// by providing a [`UserId`] (borrowed) as argument
    fn borrow(&self) -> &UserId {
        &self.user.id
    }
}

impl Player {
    pub fn new(user: User) -> Self {
        Player {
            user,
            join_datetime: Utc::now(),
        }
    }

    pub fn get_user(&self) -> &User {
        &self.user
    }

    pub fn time_elapsed_since_join(&self) -> Duration {
        let time_diff = Utc::now() - self.join_datetime;
        time_diff
    }
}
