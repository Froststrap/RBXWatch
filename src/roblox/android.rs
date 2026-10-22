use std::{sync::LazyLock, time::Duration};

use regex::Regex;
use scraper::{Html, Selector};

use super::{Deployment, Error, Platform};

const ANDROID_GOOGLE_PLAY_URL: &str =
    "https://play.google.com/store/apps/details?id=com.roblox.client";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

static VERSION_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b2\.\d+\.\d+\b").expect("valid Android version pattern"));

#[derive(Clone, Debug)]
pub struct AndroidDeploymentProvider {
    http: reqwest::Client,
}

impl Default for AndroidDeploymentProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidDeploymentProvider {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    pub async fn get_deployment(&self) -> Result<Deployment, Error> {
        let response = self
            .http
            .get(ANDROID_GOOGLE_PLAY_URL)
            .header("User-Agent", "Mozilla/5.0")
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            return Err(Error::GooglePlayStatus(status.as_u16()));
        }

        let body = response.text().await?;
        let version = find_android_version(&body)?;

        Ok(Deployment {
            platform: Platform::Android,
            channel: "LIVE".to_owned(),
            version,
            client_version_upload: String::new(),
        })
    }
}

fn find_android_version(body: &str) -> Result<String, Error> {
    let document = Html::parse_document(body);
    let body_selector = Selector::parse("body").expect("valid body selector");

    let text: String = document
        .select(&body_selector)
        .flat_map(|element| element.text())
        .collect();

    VERSION_PATTERN
        .find_iter(&text)
        .map(|version| version.as_str())
        .last()
        .map(str::to_owned)
        .ok_or(Error::AndroidVersionNotFound)
}
