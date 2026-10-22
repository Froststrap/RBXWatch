use std::{collections::HashMap, sync::Mutex};

use super::{AndroidDeploymentProvider, Client, Deployment, Error, Platform};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeploymentChange {
    pub previous: Option<Deployment>,
    pub current: Deployment,
}

#[derive(Debug)]
pub struct Tracker {
    client: Client,
    android_provider: Option<AndroidDeploymentProvider>,
    state: Mutex<HashMap<String, Deployment>>,
}

impl Tracker {
    pub fn new(client: Client, android_provider: Option<AndroidDeploymentProvider>) -> Self {
        Self {
            client,
            android_provider,
            state: Mutex::new(HashMap::new()),
        }
    }

    pub async fn check(&self) -> Result<Vec<DeploymentChange>, Error> {
        let deployments = self.get_deployments().await?;
        let mut changes = Vec::new();

        let mut state = self.state.lock().expect("tracker state poisoned");

        for deployment in deployments {
            let key = deployment_key(&deployment);
            let previous = state.insert(key, deployment.clone());

            let Some(previous) = previous else {
                continue;
            };

            if previous.version == deployment.version
                && previous.client_version_upload == deployment.client_version_upload
            {
                continue;
            }

            changes.push(DeploymentChange {
                previous: Some(previous),
                current: deployment,
            });
        }

        Ok(changes)
    }

    async fn get_deployments(&self) -> Result<Vec<Deployment>, Error> {
        let mut deployments = Vec::new();

        for platform in [Platform::WindowsPlayer, Platform::MacPlayer] {
            let channels = self
                .client
                .get_public_channels(platform)
                .await
                .map_err(|source| Error::PlatformDeployment {
                    platform,
                    source: Box::new(source),
                })?;

            deployments.extend(
                channels
                    .into_iter()
                    .filter_map(|channel| channel.deployment),
            );
        }

        if let Some(android_provider) = &self.android_provider {
            let deployment = android_provider.get_deployment().await.map_err(|source| {
                Error::AndroidDeployment {
                    source: Box::new(source),
                }
            })?;

            deployments.push(deployment);
        }

        Ok(deployments)
    }
}

pub fn deployment_key(deployment: &Deployment) -> String {
    format!("{}:{}", deployment.platform, deployment.channel)
}
