use thiserror::Error;

/// Represents an error that can occur in the GdqBot application.
#[derive(Debug, Error)]
pub enum GdqBotError {
    #[error(transparent)]
    HelixError(#[from] twitch_api::helix::ClientRequestError<reqwest::Error>),

    #[error(transparent)]
    HelixAccessError(#[from] twitch_api::twitch_oauth2::tokens::errors::AppAccessTokenError<twitch_api::client::CompatError<reqwest::Error>>),
    
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    
    #[error(transparent)]
    SerenityError(#[from] serenity::prelude::SerenityError),

    #[error("Other error: {0}")]
    Other(String),
}
