use std::time::Duration;
use serenity::{all::CreateEmbed, builder::ExecuteWebhook, http::Http, model::webhook::Webhook};
use tracing::{info, error, warn};
use twitch_api::{helix::streams::get_streams, twitch_oauth2::AppAccessToken, types, HelixClient};
use miette::Result;
use kvstore_client::{KvStoreClient, generated::{GetRequest, SetRequest}};
use tonic::transport::Channel;

mod error;
use error::GdqBotError;

// Constants
const DEFAULT_TWITCH_CHANNEL_NAME: &str = "gamesdonequick";
const DEFAULT_KVSTORE_URL: &str = "http://kvstore.binarydream.fi:50051";
const KVSTORE_KEY: &str = "gdq_game";
const POLL_RATE: Duration = Duration::from_secs(2 * 60);
const USERNAME: &str = "GDQBot";
const TWITCH_BASE_URL: &str = "https://www.twitch.tv/";
const DEFAULT_OFFLINE_THRESHOLD: u32 = 3;

#[tokio::main]
async fn main() -> Result<(), GdqBotError> {
    tracing_subscriber::fmt::init();
    #[cfg(any(test, debug_assertions))]
    dotenvy::dotenv().ok();

    let mut bot = GdqBot::new();
    bot.init_helix().await?;
    bot.init_kvstore().await?;
    match bot.get_current_game_from_db().await {
        Ok(game) => info!("Current game from DB: {}", game),
        Err(e) => warn!("Failed to get current game from DB, starting fresh: {}", e),
    };
    info!(
        "Starting bot with offline threshold of {} checks",
        bot.offline_threshold
    );

    match bot.run().await {
        Ok(()) => {
            info!("Bot finished normally");
            Ok(())
        }
        Err(GdqBotError::StreamOffline(count)) => {
            info!("Exiting gracefully - stream offline after {} checks", count);
            std::process::exit(0);
        }
        Err(GdqBotError::StreamRerun(title)) => {
            info!("Exiting gracefully - stream is a rerun: {}", title);
            std::process::exit(0);
        }
        Err(e) => {
            error!("Bot error: {:?}", e);
            Err(e)
        }
    }
}

struct GdqBot<'a> {
    channel_name: String,
    client_id: twitch_api::twitch_oauth2::ClientId,
    client_secret: twitch_api::twitch_oauth2::ClientSecret,
    access_token: Option<AppAccessToken>,
    current_game: String,
    kvstore_url: String,
    kvstore_token: String,
    kvstore_client: Option<KvStoreClient<Channel>>,
    helix_client: HelixClient<'a, reqwest::Client>,
    webhooks: Vec<String>,
    offline_count: u32,
    offline_threshold: u32,
}

trait GdqBotTrait {
    fn new() -> Self;
    async fn init_helix(&mut self) -> Result<(), GdqBotError>;
    async fn init_kvstore(&mut self) -> Result<(), GdqBotError>;
    async fn run(&mut self) -> Result<(), GdqBotError>;
    async fn get_current_game_from_db(&mut self) -> Result<String, GdqBotError>;
    async fn set_current_game_to_db(&mut self, game: &str) -> Result<(), error::GdqBotError>;
    async fn send_game_change_message(&self, game: &str, stream_title: &str) -> Result<(), error::GdqBotError>;
    async fn get_current_game_from_twitch(&mut self) -> Result<Option<String>, GdqBotError>;
}

/// Represents a GDQBot instance.
impl<'a> GdqBotTrait for GdqBot<'a> {
    /// Creates a new instance of GDQBot.
    fn new() -> Self {
        let webhook_url = std::env::var("WEBHOOK_URL").unwrap_or("".to_string());
        let offline_threshold: u32 = std::env::var("OFFLINE_CHECK_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_OFFLINE_THRESHOLD);

        GdqBot {
            channel_name: std::env::var("TWITCH_CHANNEL_NAME").unwrap_or(DEFAULT_TWITCH_CHANNEL_NAME.to_string()),
            client_id: twitch_api::twitch_oauth2::ClientId::new(std::env::var("TWITCH_CLIENT_ID").unwrap_or("".to_string())),
            client_secret: twitch_api::twitch_oauth2::ClientSecret::new(std::env::var("TWITCH_CLIENT_SECRET").unwrap_or("".to_string())),
            access_token: None,
            current_game: "".to_string(),
            kvstore_url: std::env::var("KVSTORE_URL").unwrap_or(DEFAULT_KVSTORE_URL.to_string()),
            kvstore_token: std::env::var("KVSTORE_TOKEN").unwrap_or("".to_string()),
            kvstore_client: None,
            helix_client: HelixClient::default(),
            webhooks: vec![webhook_url],
            offline_count: 0,
            offline_threshold,
        }
    }

    /// Initializes the KVStore gRPC client.
    async fn init_kvstore(&mut self) -> Result<(), GdqBotError> {
        let client = kvstore_client::connect(&self.kvstore_url).await?;
        info!("Connected to KVStore at {}", self.kvstore_url);
        self.kvstore_client = Some(client);
        Ok(())
    }

    /// Initializes the Helix client and retrieves the app access token.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the app access token cannot be retrieved.
    async fn init_helix(&mut self) -> Result<(), GdqBotError> {
        let token = AppAccessToken::get_app_access_token(
            &self.helix_client,
            self.client_id.to_owned(),
            self.client_secret.to_owned(),
            vec![], // scopes
        ).await;

        match token {
            Err(error) => {
                error!("Error: {:?}", error);
                Err(error.into())
            },
            Ok(token) => {
                info!("App access token retrieved successfully");
                self.access_token = Some(token);
                Ok(())
            }
        }
    }

    /// Starts the bot and continuously checks for game changes.
    /// Exits gracefully after consecutive offline checks exceed threshold.
    async fn run(&mut self) -> Result<(), GdqBotError> {
        let mut interval = tokio::time::interval(POLL_RATE);
        loop {
            interval.tick().await; // This should go first.
            match self.get_current_game_from_twitch().await? {
                Some(_) => {
                    // Stream is online, reset offline counter
                    self.offline_count = 0;
                }
                None => {
                    // Stream is offline
                    self.offline_count += 1;
                    info!(
                        "Stream offline check {}/{}",
                        self.offline_count, self.offline_threshold
                    );

                    if self.offline_count >= self.offline_threshold {
                        info!(
                            "Stream has been offline for {} consecutive checks. Exiting gracefully.",
                            self.offline_count
                        );
                        return Err(GdqBotError::StreamOffline(self.offline_count));
                    }
                }
            }
        }
    }

    /// Retrieves the current game from the key-value store.
    async fn get_current_game_from_db(&mut self) -> Result<String, GdqBotError> {
        let client = self.kvstore_client.as_mut()
            .ok_or_else(|| GdqBotError::Other("KVStore client not initialized".to_string()))?;

        let request = GetRequest {
            key: KVSTORE_KEY.to_string(),
            token: self.kvstore_token.clone(),
        };

        let response = client.get(request).await?.into_inner();

        if !response.found {
            return Err(GdqBotError::Other("Game not found in KVStore".to_string()));
        }

        self.current_game = response.value;
        Ok(self.current_game.clone())
    }

    /// Sets the current game in the key-value store.
    ///
    /// # Errors
    ///
    /// Returns an error if the game cannot be set in the key-value store.
    async fn set_current_game_to_db(&mut self, game: &str) -> Result<(), error::GdqBotError> {
        let client = self.kvstore_client.as_mut()
            .ok_or_else(|| GdqBotError::Other("KVStore client not initialized".to_string()))?;

        let request = SetRequest {
            key: KVSTORE_KEY.to_string(),
            value: game.to_string(),
            token: self.kvstore_token.clone(),
            ttl_seconds: None,
        };

        client.set(request).await?;
        info!("Saved game to KVStore: {}", game);

        Ok(())
    }

    /// Sends a game change message through webhooks.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the game change message cannot be sent.
    async fn send_game_change_message(&self, game: &str, stream_title: &str) -> Result<(), error::GdqBotError> {
        for webhook in self.webhooks.iter() {
            let http = Http::new("");
            let webhook = Webhook::from_url(&http, webhook).await?;
            let embed = CreateEmbed::new()
                .title("GDQ hype!")
                .description(format!("Game changed to **{}**\n*{}*\n{}{}", &game, &stream_title, &TWITCH_BASE_URL, &self.channel_name));
            let builder = ExecuteWebhook::new().embed(embed).username(USERNAME);
            webhook.execute(&http, false, builder).await?;
        }

        info!("Game changed to: {}", game);
    
        Ok(())
    }

    /// Retrieves the current game from Twitch API.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the current game cannot be retrieved from Twitch API.
    async fn get_current_game_from_twitch(&mut self) -> Result<Option<String>, GdqBotError> {
        let logins: &[&types::UserNameRef] = &[self.channel_name.as_str().into()];
        let request = get_streams::GetStreamsRequest::builder()
            .user_login(logins)
            .build();
        let response: Vec<get_streams::Stream> = self.helix_client.req_get(request, &self.access_token.clone().unwrap()).await?.data;

        if response.is_empty() {
            warn!("Stream is offline");
            return Ok(None);
        }
        let game = String::from(response.first().unwrap().game_name.as_str());
        let stream_title: String = String::from(response.first().unwrap().title.as_str());

        // Check if stream is a rerun
        if stream_title.to_lowercase().contains("rerun") {
            info!("Stream is a rerun: {}", stream_title);
            return Err(GdqBotError::StreamRerun(stream_title));
        }

        info!("Got current game from Twitch: {}", game);

        // Game name changed, save it to db and send message through webhook
        if game.ne(&self.current_game) {
            if let Err(e) = self.set_current_game_to_db(&game).await {
                error!("Failed to save game to KVStore: {}", e);
            }
            if let Err(e) = self.send_game_change_message(&game, &stream_title).await {
                error!("Failed to send game change message: {}", e);
            }
        }

        self.current_game = game;

        Ok(Some(self.current_game.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_defaults() {
        // Clear environment variables to test defaults
        std::env::remove_var("TWITCH_CHANNEL_NAME");
        std::env::remove_var("OFFLINE_CHECK_COUNT");

        let bot = GdqBot::new();

        // Test defaults that don't depend on env
        assert_eq!(bot.channel_name, "gamesdonequick");
        assert!(bot.access_token.is_none());
        assert_eq!(bot.current_game, "");
        assert_eq!(bot.offline_count, 0);
        assert_eq!(bot.offline_threshold, DEFAULT_OFFLINE_THRESHOLD);

        // Test custom offline threshold (in same test to avoid race condition)
        std::env::set_var("OFFLINE_CHECK_COUNT", "5");
        let bot2 = GdqBot::new();
        assert_eq!(bot2.offline_threshold, 5);
        std::env::remove_var("OFFLINE_CHECK_COUNT");
    }
}
