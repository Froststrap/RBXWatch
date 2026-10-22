use std::sync::Arc;

use rbxwatch::{
    bot::{Bot, BotError},
    config::Config,
};
use thiserror::Error;
use twilight_gateway::{
    CloseFrame, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _,
};

const GATEWAY_EVENTS: EventTypeFlags =
    EventTypeFlags::READY.union(EventTypeFlags::INTERACTION_CREATE);

#[derive(Debug, Error)]
enum AppError {
    #[error("failed to load configuration: {0}")]
    Config(#[from] rbxwatch::config::ConfigError),

    #[error("failed to install the rustls crypto provider")]
    CryptoProvider,

    #[error("failed to start Discord bot: {0}")]
    Bot(#[from] BotError),

    #[error("Discord gateway closed fatally")]
    GatewayClosed,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt::init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| AppError::CryptoProvider)?;

    let config = Config::load()?;

    tracing::info!("configuration loaded");

    let mut shard = Shard::new(ShardId::ONE, config.bot_token.clone(), Intents::GUILDS);
    let bot = Arc::new(Bot::new(config));

    tracing::info!("bot is running");

    let mut shutdown = Box::pin(shutdown_signal());

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                tracing::info!("shutting down");
                break;
            }
            item = shard.next_event(GATEWAY_EVENTS) => {
                let Some(item) = item else {
                    return Err(AppError::GatewayClosed);
                };

                match item {
                    Ok(Event::Ready(ready)) => {
                        bot.handle_ready(&ready).await?;
                    }
                    Ok(event) => {
                        let latency = shard
                            .latency()
                            .recent()
                            .first()
                            .copied()
                            .or_else(|| shard.latency().average());
                        let bot = Arc::clone(&bot);

                        tokio::spawn(async move {
                            bot.handle_event(event, latency).await;
                        });
                    }
                    Err(err) => {
                        tracing::warn!(?err, "error receiving event");
                    }
                }
            }
        }
    }

    shard.close(CloseFrame::NORMAL);

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
