use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{CommandContext, CommandError};

pub struct PingCommand;

impl PingCommand {
    pub const fn name(&self) -> &'static str {
        "ping"
    }

    pub const fn description(&self) -> &'static str {
        "Check whether RbxWatch is responding"
    }

    pub async fn execute(&self, context: &CommandContext) -> Result<(), CommandError> {
        let latency = context.latency.unwrap_or_default().as_millis();

        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(
                InteractionResponseDataBuilder::new()
                    .content(format!("Pong! `{latency}ms`"))
                    .build(),
            ),
        };

        context.create_response(&response).await?;

        Ok(())
    }
}
