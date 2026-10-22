use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use thiserror::Error;
use twilight_http::Client;
use twilight_model::{
    application::{
        command::{Command as ApplicationCommand, CommandOption, CommandType},
        interaction::{Interaction, InteractionData, application_command::CommandData},
    },
    http::interaction::InteractionResponse,
    id::Id,
    id::marker::ApplicationMarker,
};
use twilight_util::builder::command::CommandBuilder;

use super::{GetDeploymentInfoCommand, PingCommand};

pub struct CommandContext {
    pub http: Arc<Client>,
    pub interaction: Interaction,
    pub latency: Option<Duration>,
    responded: AtomicBool,
}

impl CommandContext {
    pub fn new(http: Arc<Client>, interaction: Interaction, latency: Option<Duration>) -> Self {
        Self {
            http,
            interaction,
            latency,
            responded: AtomicBool::new(false),
        }
    }

    pub fn application_id(&self) -> Id<ApplicationMarker> {
        self.interaction.application_id
    }

    pub fn command_data(&self) -> Option<&CommandData> {
        match self.interaction.data.as_ref()? {
            InteractionData::ApplicationCommand(data) => Some(data),
            _ => None,
        }
    }

    pub async fn create_response(
        &self,
        response: &InteractionResponse,
    ) -> Result<(), twilight_http::Error> {
        self.http
            .interaction(self.application_id())
            .create_response(self.interaction.id, &self.interaction.token, response)
            .await?;

        self.responded.store(true, Ordering::Relaxed);

        Ok(())
    }

    pub fn has_responded(&self) -> bool {
        self.responded.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("discord http request failed: {0}")]
    Http(#[from] twilight_http::Error),

    #[error("interaction did not contain application command data")]
    MissingCommandData,
}

pub enum Command {
    Ping(PingCommand),
    GetDeploymentInfo(GetDeploymentInfoCommand),
}

impl Command {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Ping(command) => command.name(),
            Self::GetDeploymentInfo(command) => command.name(),
        }
    }

    pub const fn description(&self) -> &'static str {
        match self {
            Self::Ping(command) => command.description(),
            Self::GetDeploymentInfo(command) => command.description(),
        }
    }

    pub fn application_command(&self) -> ApplicationCommand {
        let mut builder =
            CommandBuilder::new(self.name(), self.description(), CommandType::ChatInput);

        for option in self.options() {
            builder = builder.option(option);
        }

        builder.build()
    }

    fn options(&self) -> Vec<CommandOption> {
        match self {
            Self::Ping(_) => Vec::new(),
            Self::GetDeploymentInfo(command) => command.options(),
        }
    }

    pub async fn execute(&self, context: &CommandContext) -> Result<(), CommandError> {
        match self {
            Self::Ping(command) => command.execute(context).await,
            Self::GetDeploymentInfo(command) => command.execute(context).await,
        }
    }
}

pub struct Registry {
    commands: Vec<Command>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        Self {
            commands: vec![
                Command::Ping(PingCommand),
                Command::GetDeploymentInfo(GetDeploymentInfoCommand::new()),
            ],
        }
    }

    pub fn find(&self, name: &str) -> Option<&Command> {
        self.commands.iter().find(|command| command.name() == name)
    }

    pub fn application_commands(&self) -> Vec<ApplicationCommand> {
        self.commands
            .iter()
            .map(Command::application_command)
            .collect()
    }
}
