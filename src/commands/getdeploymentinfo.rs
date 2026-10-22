use std::time::Duration;

use tokio::time::timeout;
use twilight_model::{
    application::{
        command::CommandOption,
        interaction::application_command::{CommandDataOption, CommandOptionValue},
    },
    channel::message::{
        Component, Embed,
        component::{ActionRow, Button, ButtonStyle},
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
};
use twilight_util::builder::{
    command::StringBuilder,
    embed::{EmbedBuilder, EmbedFieldBuilder, ImageSource},
};

use crate::{
    commands::{CommandContext, CommandError},
    roblox::{DeploymentDetails, DeploymentService, Platform},
};

const PLAYER_ICON_URL: &str = concat!(
    "https://raw.githubusercontent.com/",
    "Froststrap/roblox-tracker-bot/main/assets/robloxPlayer.png"
);
const STUDIO_ICON_URL: &str = concat!(
    "https://raw.githubusercontent.com/",
    "Froststrap/roblox-tracker-bot/main/assets/robloxStudio.png"
);

const DEFAULT_EMBED_COLOR: u32 = 0x33_5F_FF;
const ANDROID_EMBED_COLOR: u32 = 0x3D_DC_84;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

pub struct GetDeploymentInfoCommand {
    deployment_service: DeploymentService,
}

impl Default for GetDeploymentInfoCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl GetDeploymentInfoCommand {
    pub fn new() -> Self {
        Self {
            deployment_service: DeploymentService::new(),
        }
    }

    pub const fn name(&self) -> &'static str {
        "getdeploymentinfo"
    }

    pub const fn description(&self) -> &'static str {
        "Get the current Roblox deployment information"
    }

    pub fn options(&self) -> Vec<CommandOption> {
        vec![
            StringBuilder::new("platform", "Roblox platform")
                .required(true)
                .choices(Platform::ALL.map(|platform| (platform.label(), platform.as_str())))
                .build(),
            StringBuilder::new("channel", "Roblox channel").build(),
        ]
    }

    pub async fn execute(&self, context: &CommandContext) -> Result<(), CommandError> {
        let data = context
            .command_data()
            .ok_or(CommandError::MissingCommandData)?;

        let platform_value = get_option_string(&data.options, "platform").unwrap_or_default();
        let channel = get_option_string(&data.options, "channel")
            .map(str::trim)
            .filter(|channel| !channel.is_empty())
            .unwrap_or("LIVE")
            .to_owned();

        self.defer(context).await?;

        let platform = match platform_value.parse::<Platform>() {
            Ok(platform) => platform,
            Err(err) => {
                tracing::error!(%err, platform = platform_value, "failed to get deployment info");
                return self
                    .edit_content(
                        context,
                        "Could not get deployment information. \
                         Check the platform and channel and try again.",
                    )
                    .await;
            }
        };

        let details = match timeout(
            REQUEST_TIMEOUT,
            self.deployment_service
                .get_deployment_details(platform, &channel),
        )
        .await
        {
            Ok(Ok(details)) => details,
            Ok(Err(err)) => {
                tracing::error!(
                    %platform,
                    %channel,
                    %err,
                    "failed to get deployment info",
                );

                return self
                    .edit_content(
                        context,
                        "Could not get deployment information. \
                         Check the platform and channel and try again.",
                    )
                    .await;
            }
            Err(_) => {
                tracing::error!(
                    %platform,
                    %channel,
                    "timed out while getting deployment info",
                );

                return self
                    .edit_content(
                        context,
                        "Could not get deployment information. \
                         Check the platform and channel and try again.",
                    )
                    .await;
            }
        };

        let embed = build_deployment_embed(platform, &details);
        let components = build_download_components(&details.download_links);

        context
            .http
            .interaction(context.application_id())
            .update_response(&context.interaction.token)
            .embeds(Some(std::slice::from_ref(&embed)))
            .components(Some(&components))
            .await?;

        Ok(())
    }

    async fn defer(&self, context: &CommandContext) -> Result<(), CommandError> {
        let response = InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: None,
        };

        context.create_response(&response).await?;

        Ok(())
    }

    async fn edit_content(
        &self,
        context: &CommandContext,
        content: &str,
    ) -> Result<(), CommandError> {
        context
            .http
            .interaction(context.application_id())
            .update_response(&context.interaction.token)
            .content(Some(content))
            .await?;

        Ok(())
    }
}

fn get_platform_icon_url(platform: Platform) -> &'static str {
    match platform {
        Platform::WindowsStudio | Platform::MacStudio => STUDIO_ICON_URL,
        _ => PLAYER_ICON_URL,
    }
}

const fn get_platform_embed_color(platform: Platform) -> u32 {
    if matches!(platform, Platform::Android) {
        ANDROID_EMBED_COLOR
    } else {
        DEFAULT_EMBED_COLOR
    }
}

fn build_deployment_embed(platform: Platform, details: &DeploymentDetails) -> Embed {
    let deployment = &details.deployment;

    let mut embed = EmbedBuilder::new()
        .title(format!("Current version for {platform}"))
        .description(format!(
            "The `{}` channel's current deployment for `{}`.",
            deployment.channel, platform
        ))
        .color(get_platform_embed_color(platform))
        .thumbnail(
            ImageSource::url(get_platform_icon_url(platform)).expect("valid platform icon url"),
        );

    if !details.client_settings_url.is_empty() {
        embed = embed.url(details.client_settings_url.clone());
    }

    if let Some(built_at) = details.built_at {
        let unix = built_at.timestamp();

        embed = embed.field(EmbedFieldBuilder::new(
            "Built on:",
            format!("<t:{unix}:F> (<t:{unix}:R>)\nFound in deployment history"),
        ));
    }

    embed = embed.field(
        EmbedFieldBuilder::new("Version:", format!("```{}```", deployment.version)).inline(),
    );

    if !deployment.client_version_upload.is_empty() {
        embed = embed.field(
            EmbedFieldBuilder::new(
                "GUID/Hash:",
                format!("```{}```", deployment.client_version_upload),
            )
            .inline(),
        );
    }

    embed.build()
}

fn build_download_components(links: &[crate::roblox::DownloadLink]) -> Vec<Component> {
    if links.is_empty() {
        return Vec::new();
    }

    let buttons = links
        .iter()
        .map(|link| {
            Component::Button(Button {
                id: None,
                custom_id: None,
                disabled: false,
                emoji: None,
                label: Some(format!("Download ({})", link.name)),
                style: ButtonStyle::Link,
                url: Some(link.url.clone()),
                sku_id: None,
            })
        })
        .collect();

    vec![Component::ActionRow(ActionRow {
        id: None,
        components: buttons,
    })]
}

fn get_option_string<'a>(options: &'a [CommandDataOption], name: &str) -> Option<&'a str> {
    options
        .iter()
        .find(|option| option.name == name)
        .and_then(|option| match &option.value {
            CommandOptionValue::String(value) => Some(value.as_str()),
            _ => None,
        })
}
