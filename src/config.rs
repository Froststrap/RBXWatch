use std::{env, num::ParseIntError};

use thiserror::Error;
use twilight_model::id::{Id, marker::GuildMarker};

#[derive(Clone, Debug)]
pub struct Config {
    pub bot_token: String,
    pub discord_guild_id: Option<Id<GuildMarker>>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("BOT_TOKEN is not set")]
    MissingBotToken,

    #[error("DISCORD_GUILD_ID is not a valid guild id")]
    InvalidGuildId(#[from] ParseIntError),
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        if let Err(err) = dotenvy::dotenv() {
            tracing::info!(%err, "no .env file found, using environment variables");
        }

        let bot_token = env::var("BOT_TOKEN")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .ok_or(ConfigError::MissingBotToken)?;

        let discord_guild_id = env::var("DISCORD_GUILD_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().parse::<u64>())
            .transpose()?
            .map(Id::new);

        Ok(Self {
            bot_token,
            discord_guild_id,
        })
    }
}
