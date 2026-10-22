use std::{sync::Arc, time::Duration};

use thiserror::Error;
use tracing::{error, info, warn};
use twilight_gateway::Event;
use twilight_http::Client;
use twilight_model::{
    application::interaction::{InteractionData, InteractionType},
    channel::message::MessageFlags,
    gateway::payload::incoming::Ready,
    http::interaction::{InteractionResponse, InteractionResponseType},
    id::{Id, marker::ApplicationMarker},
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::{
    commands::{CommandContext, Registry},
    config::Config,
};

#[derive(Debug, Error)]
pub enum BotError {
    #[error("discord http request failed: {0}")]
    Http(#[from] twilight_http::Error),
}

pub struct Bot {
    http: Arc<Client>,
    registry: Arc<Registry>,
    config: Config,
}

impl Bot {
    pub fn new(config: Config) -> Self {
        Self {
            http: Arc::new(Client::new(config.bot_token.clone())),
            registry: Arc::new(Registry::new()),
            config,
        }
    }

    pub async fn handle_event(self: &Arc<Self>, event: Event, latency: Option<Duration>) {
        if let Err(err) = self.dispatch(event, latency).await {
            error!(%err, "handler failed");
        }
    }

    pub async fn handle_ready(&self, ready: &Ready) -> Result<(), BotError> {
        info!(
            user = ready.user.name,
            userId = ready.user.id.get(),
            "Discord bot is online",
        );

        self.register_application_commands(ready.application.id)
            .await?;

        Ok(())
    }

    async fn dispatch(&self, event: Event, latency: Option<Duration>) -> Result<(), BotError> {
        match event {
            Event::Ready(ready) => {
                self.handle_ready(&ready).await?;
            }

            Event::InteractionCreate(interaction) => {
                let interaction = interaction.0;

                if interaction.kind != InteractionType::ApplicationCommand {
                    return Ok(());
                }

                let command_name = {
                    let Some(InteractionData::ApplicationCommand(data)) = interaction.data.as_ref()
                    else {
                        return Ok(());
                    };

                    data.name.clone()
                };

                let Some(command) = self.registry.find(&command_name) else {
                    return Ok(());
                };

                let user_id = interaction
                    .member
                    .as_ref()
                    .and_then(|member| member.user.as_ref())
                    .map(|user| user.id.get())
                    .or_else(|| interaction.user.as_ref().map(|user| user.id.get()));

                info!(
                    command = command_name,
                    ?user_id,
                    guildId = interaction.guild_id.map(Id::get),
                    "received application command",
                );

                let context = CommandContext::new(Arc::clone(&self.http), interaction, latency);

                if let Err(err) = command.execute(&context).await {
                    error!(command = command_name, %err, "application command failed");

                    if let Err(err) = self.send_command_error(&context).await {
                        warn!(%err, "failed to send command error response");
                    }
                }
            }

            _ => {}
        }

        Ok(())
    }

    async fn register_application_commands(
        &self,
        application_id: Id<ApplicationMarker>,
    ) -> Result<(), twilight_http::Error> {
        let commands = self.registry.application_commands();
        let interaction_client = self.http.interaction(application_id);

        match self.config.discord_guild_id {
            Some(guild_id) => {
                interaction_client
                    .set_guild_commands(guild_id, &commands)
                    .await?;
            }
            None => {
                interaction_client.set_global_commands(&commands).await?;
            }
        }

        info!(
            count = commands.len(),
            guildId = self.config.discord_guild_id.map(Id::get),
            "synchronised application commands",
        );

        Ok(())
    }

    async fn send_command_error(
        &self,
        context: &CommandContext,
    ) -> Result<(), twilight_http::Error> {
        let content = "An error occurred while executing that command.";

        if context.has_responded() {
            self.http
                .interaction(context.application_id())
                .update_response(&context.interaction.token)
                .content(Some(content))
                .await?;

            return Ok(());
        }

        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(
                InteractionResponseDataBuilder::new()
                    .content(content)
                    .flags(MessageFlags::EPHEMERAL)
                    .build(),
            ),
        };

        context.create_response(&response).await?;

        Ok(())
    }
}
