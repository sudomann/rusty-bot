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
