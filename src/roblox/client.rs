use std::time::Duration;

use serde::de::DeserializeOwned;

use super::Error;

pub(crate) const CLIENT_SETTINGS_CDN_BASE_URL: &str = "https://clientsettingscdn.roblox.com";
pub(crate) const CLIENT_SETTINGS_BASE_URL: &str = "https://clientsettings.roblox.com";
pub(crate) const DEFAULT_CHANNEL: &str = "production";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug)]
pub struct Client {
    http: reqwest::Client,
    channel_token: Option<String>,
    cdn_base_url: String,
    fallback_base_url: String,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            channel_token: None,
            cdn_base_url: CLIENT_SETTINGS_CDN_BASE_URL.to_owned(),
            fallback_base_url: CLIENT_SETTINGS_BASE_URL.to_owned(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_base_url(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();

        Self {
            cdn_base_url: base_url.clone(),
            fallback_base_url: base_url,
            ..Self::new()
        }
    }

    pub fn set_channel_token(&mut self, token: impl Into<String>) {
        self.channel_token = Some(token.into());
    }

    pub(crate) async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        match self.get_json_from_base(&self.cdn_base_url, path).await {
            Ok(value) => Ok(value),
            Err(err) => {
                if matches!(err.status_code(), Some(401 | 403 | 404)) {
                    return Err(err);
                }

                self.get_json_from_base(&self.fallback_base_url, path).await
            }
        }
    }

    async fn get_json_from_base<T: DeserializeOwned>(
        &self,
        base_url: &str,
        path: &str,
    ) -> Result<T, Error> {
        let url = format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        let mut request = self.http.get(url).timeout(REQUEST_TIMEOUT);

        if let Some(channel_token) = &self.channel_token {
            request = request.header("Roblox-Channel-Token", channel_token);
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            return Err(Error::HttpStatus(status.as_u16()));
        }

        let body = response.text().await?;

        Ok(serde_json::from_str(&body)?)
    }
}

pub(crate) fn normalize_channel(channel: &str) -> String {
    if channel.eq_ignore_ascii_case("live") || channel.eq_ignore_ascii_case("zlive") {
        DEFAULT_CHANNEL.to_owned()
    } else {
        channel.to_owned()
    }
}

pub(crate) fn build_channel_path(binary_type: &str, channel: &str) -> String {
    let channel = normalize_channel(channel);

    let mut path = format!("/v2/client-version/{binary_type}");

    if !channel.eq_ignore_ascii_case(DEFAULT_CHANNEL) {
        path.push_str("/channel/");
        path.push_str(&url_encode_path_segment(&channel));
    }

    path
}

fn url_encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{byte:02X}"));
            }
        }
    }

    encoded
}

pub fn is_invalid_channel_status(status_code: u16) -> bool {
    matches!(status_code, 401 | 403 | 404)
}
