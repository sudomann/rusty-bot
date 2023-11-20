use thiserror::Error as ThisError;
/// Error type to wrap errors that will often be a possibility in tandem:
///
/// [`mongodb::error::Error`]
///
/// [`serenity::Error`]
///
/// This is to avoid repetitive verbose error handling for both types in
/// command handlers by facilitating/enabling the use of the `?` operator on [`Result`] returns.
#[derive(ThisError, Debug)]
pub enum RustyError {
    #[error("mongodb returned an error")]
    Mongo(mongodb::error::Error),
    #[error("serenity returned an error")]
    Serenity(serenity::Error),
}

impl From<mongodb::error::Error> for RustyError {
    fn from(error: mongodb::error::Error) -> Self {
        RustyError::Mongo(error)
    }
}

impl From<serenity::Error> for RustyError {
    fn from(error: serenity::Error) -> Self {
        RustyError::Serenity(error)
    }
}

pub type Error = RustyError;

#[derive(ThisError, Debug)]
pub enum SetCaptainErr {
    #[error("User is already a captain")]
    IsCaptainAlready,
    #[error("There are no captain spots available")]
    CaptainSpotsFilled,
    #[error("User is not a participant in pug")]
    ForeignUser,
    #[error("An error occured when trying to communicate with the database")]
    MongoError(mongodb::error::Error),
    #[error("An invalid result was encountered while checking for available captain spots")]
    InvalidCount,
    #[error("The thread id provided did not yield a valid picking session with players")]
    NoPlayers,
    #[error(
        "It seems captaining operations were executed without error but completed \
    but the current state is unexpected"
    )]
    Unknown,
}
