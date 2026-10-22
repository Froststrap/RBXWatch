use chrono::{DateTime, Utc};

use super::{
    AndroidDeploymentProvider, Client, DeployHistoryProvider, Deployment, DownloadLink, Error,
    Platform, client_settings_url, download_links, links::is_production_channel,
};

#[derive(Clone, Debug)]
pub struct DeploymentDetails {
    pub deployment: Deployment,
    pub client_settings_url: String,
    pub built_at: Option<DateTime<Utc>>,
    pub download_links: Vec<DownloadLink>,
}

#[derive(Clone, Debug)]
pub struct DeploymentService {
    client: Client,
    android_provider: AndroidDeploymentProvider,
    history_provider: DeployHistoryProvider,
}

impl Default for DeploymentService {
    fn default() -> Self {
        Self::new()
    }
}

impl DeploymentService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            android_provider: AndroidDeploymentProvider::new(),
            history_provider: DeployHistoryProvider::new(),
        }
    }

    pub async fn get_deployment(
        &self,
        platform: Platform,
        channel: &str,
    ) -> Result<Deployment, Error> {
        if platform == Platform::Android {
            return self
                .android_provider
                .get_deployment()
                .await
                .map_err(|source| Error::AndroidDeployment {
                    source: Box::new(source),
                });
        }

        self.client
            .get_deployment(platform, channel)
            .await
            .map_err(|source| Error::PlatformDeployment {
                platform,
                source: Box::new(source),
            })
    }

    pub async fn get_deployment_details(
        &self,
        platform: Platform,
        channel: &str,
    ) -> Result<DeploymentDetails, Error> {
        let deployment = self.get_deployment(platform, channel).await?;

        let mut details = DeploymentDetails {
            client_settings_url: client_settings_url(platform, &deployment.channel),
            deployment,
            built_at: None,
            download_links: Vec::new(),
        };

        if !is_production_channel(&details.deployment.channel) {
            return Ok(details);
        }

        details.download_links = download_links(platform, &details.deployment);

        match self
            .history_provider
            .get_build_time(platform, &details.deployment.version)
            .await
        {
            Ok(built_at) => details.built_at = Some(built_at),
            Err(Error::BuildTimeNotFound) => {}
            Err(err) => {
                tracing::warn!(
                    %platform,
                    %err,
                    "could not read deploy history",
                );
            }
        }

        Ok(details)
    }
}
