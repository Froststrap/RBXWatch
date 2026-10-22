use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use super::{
    Client, Error,
    client::{build_channel_path, normalize_channel},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum Platform {
    #[serde(rename = "WindowsPlayer")]
    WindowsPlayer,
    #[serde(rename = "MacPlayer")]
    MacPlayer,
    #[serde(rename = "WindowsStudio64")]
    WindowsStudio,
    #[serde(rename = "MacStudio")]
    MacStudio,
    #[serde(rename = "Android")]
    Android,
}

impl Platform {
    pub const ALL: [Self; 5] = [
        Self::WindowsPlayer,
        Self::MacPlayer,
        Self::WindowsStudio,
        Self::MacStudio,
        Self::Android,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WindowsPlayer => "WindowsPlayer",
            Self::MacPlayer => "MacPlayer",
            Self::WindowsStudio => "WindowsStudio64",
            Self::MacStudio => "MacStudio",
            Self::Android => "Android",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::WindowsPlayer => "Windows Player",
            Self::MacPlayer => "Mac Player",
            Self::WindowsStudio => "Windows Studio",
            Self::MacStudio => "Mac Studio",
            Self::Android => "Android",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for Platform {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|platform| platform.as_str() == value)
            .ok_or_else(|| Error::UnsupportedPlatform(value.to_owned()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Deployment {
    pub platform: Platform,
    pub channel: String,
    pub version: String,
    #[serde(
        rename = "clientVersionUpload",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub client_version_upload: String,
}

impl Client {
    pub async fn get_deployment(
        &self,
        platform: Platform,
        channel: &str,
    ) -> Result<Deployment, Error> {
        let channel = if channel.is_empty() { "LIVE" } else { channel };

        let binary_type = match platform {
            Platform::WindowsPlayer => "WindowsPlayer",
            Platform::MacPlayer => "MacPlayer",
            Platform::WindowsStudio => "WindowsStudio64",
            Platform::MacStudio => "MacStudio",
            Platform::Android => return Err(Error::AndroidProvider),
        };

        self.get_client_deployment(platform, binary_type, channel)
            .await
    }

    pub async fn get_windows_deployment(&self) -> Result<Deployment, Error> {
        self.get_deployment(Platform::WindowsPlayer, "LIVE").await
    }

    pub async fn get_windows_deployment_for_channel(
        &self,
        channel: &str,
    ) -> Result<Deployment, Error> {
        self.get_deployment(Platform::WindowsPlayer, channel).await
    }

    pub async fn get_mac_deployment(&self) -> Result<Deployment, Error> {
        self.get_deployment(Platform::MacPlayer, "LIVE").await
    }

    pub async fn get_mac_deployment_for_channel(&self, channel: &str) -> Result<Deployment, Error> {
        self.get_deployment(Platform::MacPlayer, channel).await
    }

    async fn get_client_deployment(
        &self,
        platform: Platform,
        binary_type: &str,
        channel: &str,
    ) -> Result<Deployment, Error> {
        if channel.is_empty() {
            return Err(Error::EmptyChannel);
        }

        let api_channel = normalize_channel(channel);

        let response: ClientVersionResponse = self
            .get_json(&build_channel_path(binary_type, &api_channel))
            .await
            .map_err(|source| Error::Deployment {
                platform,
                channel: channel.to_owned(),
                source: Box::new(source),
            })?;

        Ok(Deployment {
            platform,
            channel: channel.to_owned(),
            version: response.version,
            client_version_upload: response.client_version_upload,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ClientVersionResponse {
    version: String,
    #[serde(rename = "clientVersionUpload", default)]
    client_version_upload: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_platforms() {
        for platform in Platform::ALL {
            assert_eq!(platform.as_str().parse::<Platform>().ok(), Some(platform));
        }

        assert!("Unknown".parse::<Platform>().is_err());
    }

    #[test]
    fn builds_channel_paths() {
        assert_eq!(
            build_channel_path("WindowsPlayer", "LIVE"),
            "/v2/client-version/WindowsPlayer"
        );
        assert_eq!(
            build_channel_path("WindowsPlayer", "zLive"),
            "/v2/client-version/WindowsPlayer"
        );
        assert_eq!(
            build_channel_path("WindowsPlayer", "zCanary"),
            "/v2/client-version/WindowsPlayer/channel/zCanary"
        );
        assert_eq!(
            build_channel_path("MacStudio", "channel with space"),
            "/v2/client-version/MacStudio/channel/channel%20with%20space"
        );
    }
}
