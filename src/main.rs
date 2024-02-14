use std::time::Duration;
use serenity::{builder::ExecuteWebhook, http::Http, model::webhook::Webhook};
use twitch_api2::{twitch_oauth2::AppAccessToken, HelixClient, helix::streams::get_streams};

mod error;
use error::GdqBotError;

// Constants
const KVSTORE_URL: &str = "https://kvstore.binarydream.fi/";
const KVSTORE_KEY: &str = "gdq_game";
const POLL_RATE: Duration = Duration::from_secs(2 * 60);
const USERNAME: &str = "GDQBot";
const GDQ_TWITCH_URL: &str = "https://www.twitch.tv/gamesdonequick";

#[tokio::main]
async fn main() -> Result<(), twitch_api2::twitch_oauth2::tokens::errors::AppAccessTokenError<twitch_api2::client::CompatError<reqwest::Error>>>{
    dotenv::dotenv().ok();
    let mut bot = GdqBot::new();
    bot.init_helix().await?;
    bot.get_current_game_from_db().await;
    println!("Current game: {}", bot.current_game.clone());
    println!("Starting bot");
    bot.run().await;


    Ok(())
}

/// Represents a request to the key-value store.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct KVStoreRequest {
    value: String,
}

struct GdqBot {
    client_id: twitch_api2::twitch_oauth2::ClientId,
    client_secret: twitch_api2::twitch_oauth2::ClientSecret,
    access_token: Option<AppAccessToken>,
    current_game: String,
    channel_id: String,
    http_client: reqwest::Client,
    kvstore_token: String,
    helix_client: HelixClient::<'static, reqwest::Client>,
    webhooks: Vec<String>,
}

/// Represents a GDQBot instance.
///
/// The GDQBot struct contains the necessary fields and methods to interact with Twitch API,
/// retrieve and store data in a key-value store, and send game change messages through webhooks.
///
/// # Fields
///
/// - `client_id`: The Twitch client ID.
/// - `client_secret`: The Twitch client secret.
/// - `access_token`: The access token for Twitch API.
/// - `current_game`: The current game being played.
/// - `channel_id`: The ID of the Twitch channel.
/// - `http_client`: The HTTP client for making requests.
/// - `kvstore_token`: The token for accessing the key-value store.
/// - `helix_client`: The Helix client for interacting with Twitch API.
/// - `webhooks`: The list of webhooks to send game change messages to.
///
/// # Methods
///
/// - `new`: Creates a new instance of GDQBot.
/// - `init_helix`: Initializes the Helix client and retrieves the app access token.
/// - `run`: Starts the bot and continuously checks for game changes.
/// - `get_current_game_from_db`: Retrieves the current game from the key-value store.
/// - `set_current_game_to_db`: Sets the current game in the key-value store.
/// - `send_game_change_message`: Sends a game change message through webhooks.
/// - `get_current_game_from_twitch`: Retrieves the current game from Twitch API.
impl GdqBot {
    /// Creates a new instance of GDQBot.
    fn new() -> Self {
        let webhook_url = std::env::var("WEBHOOK_URL").unwrap_or("".to_string());
        
        GdqBot {
            client_id: twitch_api2::twitch_oauth2::ClientId::new(std::env::var("TWITCH_CLIENT_ID").unwrap_or("".to_string())),
            client_secret: twitch_api2::twitch_oauth2::ClientSecret::new(std::env::var("TWITCH_CLIENT_SECRET").unwrap_or("".to_string())),
            access_token: None,
            current_game: String::from(""),
            channel_id: String::from(std::env::var("CHANNEL_ID").unwrap_or("".to_string())),
            http_client: reqwest::Client::new(),
            kvstore_token: String::from(std::env::var("KVSTORE_TOKEN").unwrap_or("".to_string())),
            helix_client: HelixClient::<'static, reqwest::Client>::default(),
            webhooks: vec![webhook_url],
        }

    }

    /// Initializes the Helix client and retrieves the app access token.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the app access token cannot be retrieved.
    async fn init_helix(&mut self) -> Result<(), twitch_api2::twitch_oauth2::tokens::errors::AppAccessTokenError<twitch_api2::client::CompatError<reqwest::Error>>> {
        let token = AppAccessToken::get_app_access_token(
            &self.helix_client,
            self.client_id.to_owned(),
            self.client_secret.to_owned(),
            vec![], // scopes
        ).await;

        if let Err(error) = token {
            println!("Error: {:?}", error);
            return Err(error);
        }

        self.access_token = Some(token.unwrap());

        Ok(())
    }

    /// Starts the bot and continuously checks for game changes.
    async fn run(&mut self) {
        let mut interval = tokio::time::interval(POLL_RATE);
        loop {
            interval.tick().await; // This should go first.
            let _ = self.get_current_game_from_twitch().await;
        }
    }

    /// Retrieves the current game from the key-value store.
    async fn get_current_game_from_db(&mut self) -> String {
        let response = self.http_client.get(format!("{}{}", KVSTORE_URL, KVSTORE_KEY).as_str())
            .bearer_auth(&self.kvstore_token)
            .send().await.unwrap();

        if response.status() != 200 {
            return String::from("");
        }
        
        self.current_game = response.json().await.unwrap();

        return self.current_game.clone();
    }

    /// Sets the current game in the key-value store.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the game cannot be set in the key-value store.
    async fn set_current_game_to_db(&self, game: &String) -> Result<(), error::GdqBotError> {
        let body = KVStoreRequest {
            value: game.to_string(),
        };

        let response = self.http_client.post(format!("{}{}", KVSTORE_URL, KVSTORE_KEY).as_str())
            .bearer_auth(&self.kvstore_token)
            .json(&body)
            .send().await?;

        if response.status() != 200 {
            return Err("Error setting game to KVStore".into());
        }

        println!("Saved game to KVStore: {}", game);

        Ok(())
    }

    /// Sends a game change message through webhooks.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the game change message cannot be sent.
    async fn send_game_change_message(&self, game: &str, stream_title: &str) -> Result<(), error::GdqBotError>{
        for webhook in self.webhooks.iter() {
            let http = Http::new("");
            let webhook = Webhook::from_url(&http, webhook).await?;
            let builder = ExecuteWebhook::new().embed(
                serenity::builder::CreateEmbed::default()
                    .title("GDQ hype!")
                    .description(format!("Game changed to **{}**\n*{}*\n{}", game, stream_title, GDQ_TWITCH_URL)
                    .as_str())
                    .color(0x00FF00)
            ).username(USERNAME);
            webhook
                .execute(&http, true, builder).await?;
        }
        println!("Game changed to: {}", game);
    
        Ok(())
    }

    /// Retrieves the current game from Twitch API.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the current game cannot be retrieved from Twitch API.
    async fn get_current_game_from_twitch(&mut self) -> Result<Option<String>, GdqBotError> {
        let request = get_streams::GetStreamsRequest::builder()
            .user_id(vec![self.channel_id.clone().into()])
            .build();
        let response: Vec<get_streams::Stream> = self.helix_client.req_get(request, &self.access_token.clone().unwrap()).await?.data;

        if response.is_empty() {
            println!("Error: stream is offline");
            return Ok(None);
        }
        let game = String::from(response.first().unwrap().game_name.as_str());
        let stream_title: String = String::from(response.first().unwrap().title.as_str());

        println!("Got current game from Twitch: {}", game);

        // Game name changed, save it to db and send message through webhook
        if game.ne(&self.current_game) {
            let _ = tokio::join!(
                self.set_current_game_to_db(&game),
                self.send_game_change_message(&game, &stream_title)
            );
        }

        self.current_game = game;

        Ok(Some(self.current_game.clone()))
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // TODO
}