use thiserror::Error;

/// Represents an error that can occur in the GdqBot application.
#[derive(Debug, Error)]
pub enum GdqBotError {
    #[error(transparent)]
    HelixError(Box<twitch_api::helix::ClientRequestError<reqwest::Error>>),

    #[error(transparent)]
    HelixAccessError(Box<twitch_api::twitch_oauth2::tokens::errors::AppAccessTokenError<twitch_api::client::CompatError<reqwest::Error>>>),
    
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    
    #[error(transparent)]
    SerenityError(Box<serenity::prelude::SerenityError>),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<twitch_api::helix::ClientRequestError<reqwest::Error>> for GdqBotError {
    fn from(error: twitch_api::helix::ClientRequestError<reqwest::Error>) -> Self {
        Self::HelixError(Box::new(error))
    }
}

impl From<twitch_api::twitch_oauth2::tokens::errors::AppAccessTokenError<twitch_api::client::CompatError<reqwest::Error>>> for GdqBotError {
    fn from(error: twitch_api::twitch_oauth2::tokens::errors::AppAccessTokenError<twitch_api::client::CompatError<reqwest::Error>>) -> Self {
        Self::HelixAccessError(Box::new(error))
    }
}

impl From<serenity::prelude::SerenityError> for GdqBotError {
    fn from(error: serenity::prelude::SerenityError) -> Self {
        Self::SerenityError(Box::new(error))
    }
}
