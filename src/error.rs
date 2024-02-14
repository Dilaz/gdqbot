use core::fmt;

/// Represents an error that can occur in the GdqBot application.
#[derive(Debug, Clone)]
pub struct GdqBotError {
    message: String,
}

impl GdqBotError {
    /// Creates a new instance of `GdqBotError` with the given error message.
    fn new(message: &str) -> GdqBotError {
        GdqBotError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for GdqBotError {
    /// Formats the error message for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GdqBotError: {}", self.message)
    }
}

impl From<twitch_api2::helix::ClientRequestError<reqwest::Error>> for GdqBotError {
    /// Converts a `twitch_api2::helix::ClientRequestError<reqwest::Error>` into a `GdqBotError`.
    fn from(error: twitch_api2::helix::ClientRequestError<reqwest::Error>) -> Self {
        GdqBotError::new(&format!("ClientRequestError: {:?}", error))
    }
}

impl From<&str> for GdqBotError {
    /// Converts a string slice into a `GdqBotError`.
    fn from(error: &str) -> Self {
        GdqBotError::new(error)
    }
}

impl From<reqwest::Error> for GdqBotError {
    /// Converts a `reqwest::Error` into a `GdqBotError`.
    fn from(error: reqwest::Error) -> Self {
        GdqBotError::new(&format!("ReqwestError: {:?}", error))
    }
}

impl From<serenity::prelude::SerenityError> for GdqBotError {
    /// Converts a `serenity::prelude::SerenityError` into a `GdqBotError`.
    fn from(error: serenity::prelude::SerenityError) -> Self {
        GdqBotError::new(&format!("SerenityError: {:?}", error))
    }
}
