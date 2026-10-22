use thiserror::Error;

use super::Platform;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Roblox API returned HTTP {0}")]
    HttpStatus(u16),

    #[error("send request: {0}")]
    Request(#[from] reqwest::Error),

    #[error("decode response: {0}")]
    Decode(#[from] serde_json::Error),

    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("Android deployments use the Android deployment provider")]
    AndroidProvider,

    #[error("channel cannot be empty")]
    EmptyChannel,

    #[error("get {platform} deployment for channel {channel}: {source}")]
    Deployment {
        platform: Platform,
        channel: String,
        #[source]
        source: Box<Error>,
    },

    #[error("get {platform} deployment: {source}")]
    PlatformDeployment {
        platform: Platform,
        #[source]
        source: Box<Error>,
    },

    #[error("check channel {channel}: {source}")]
    ChannelCheck {
        channel: String,
        #[source]
        source: Box<Error>,
    },

    #[error("get Android deployment: {source}")]
    AndroidDeployment {
        #[source]
        source: Box<Error>,
    },

    #[error("could not find Roblox Android version on Google Play")]
    AndroidVersionNotFound,

    #[error("Google Play returned HTTP {0}")]
    GooglePlayStatus(u16),

    #[error("build time not found in deploy history")]
    BuildTimeNotFound,
}

impl Error {
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::HttpStatus(status_code) => Some(*status_code),
            Self::Deployment { source, .. }
            | Self::PlatformDeployment { source, .. }
            | Self::ChannelCheck { source, .. }
            | Self::AndroidDeployment { source, .. } => source.status_code(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_status_code_through_wrapped_errors() {
        let error = Error::ChannelCheck {
            channel: "zCanary".to_owned(),
            source: Box::new(Error::Deployment {
                platform: Platform::WindowsPlayer,
                channel: "zCanary".to_owned(),
                source: Box::new(Error::HttpStatus(401)),
            }),
        };

        assert_eq!(error.status_code(), Some(401));
    }

    #[test]
    fn status_code_is_none_without_http_status() {
        assert_eq!(Error::EmptyChannel.status_code(), None);
    }
}
