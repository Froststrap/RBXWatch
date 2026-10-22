mod android;
mod channels;
mod client;
mod deploy_history;
mod deployment_service;
mod deployments;
mod error;
mod links;
mod tracker;

pub use self::{
    android::AndroidDeploymentProvider,
    channels::{Channel, known_channels},
    client::{Client, is_invalid_channel_status},
    deploy_history::DeployHistoryProvider,
    deployment_service::{DeploymentDetails, DeploymentService},
    deployments::{Deployment, Platform},
    error::Error,
    links::{DownloadLink, client_settings_url, download_links, is_production_channel},
    tracker::{DeploymentChange, Tracker, deployment_key},
};
