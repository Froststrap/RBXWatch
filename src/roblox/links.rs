use super::{
    Deployment, Platform,
    client::{
        CLIENT_SETTINGS_CDN_BASE_URL, DEFAULT_CHANNEL, build_channel_path, normalize_channel,
    },
};

pub(crate) const SETUP_CDN_BASE_URL: &str = "https://setup.rbxcdn.com";
const RDD_BASE_URL: &str = "https://rdd.latte.to/";
const APK_MIRROR_BASE_URL: &str = "https://www.apkmirror.com/apk/roblox-corporation/roblox/";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadLink {
    pub name: String,
    pub url: String,
}

pub fn is_production_channel(channel: &str) -> bool {
    normalize_channel(channel).eq_ignore_ascii_case(DEFAULT_CHANNEL)
}

pub fn client_settings_url(platform: Platform, channel: &str) -> String {
    if platform == Platform::Android {
        return String::new();
    }

    format!(
        "{}{}",
        CLIENT_SETTINGS_CDN_BASE_URL,
        build_channel_path(platform.as_str(), channel)
    )
}

fn build_rdd_url(platform: Platform, version_guid: &str) -> String {
    format!(
        "{RDD_BASE_URL}?binaryType={}&channel=LIVE&version={version_guid}",
        platform.as_str()
    )
}

fn build_apk_mirror_url(version: &str) -> String {
    let slug = version.replace('.', "-");

    format!("{APK_MIRROR_BASE_URL}roblox-{slug}-release/")
}

pub fn download_links(platform: Platform, deployment: &Deployment) -> Vec<DownloadLink> {
    let version_guid = &deployment.client_version_upload;

    match platform {
        Platform::WindowsPlayer => {
            if version_guid.is_empty() {
                return Vec::new();
            }

            vec![DownloadLink {
                name: "rdd.latte.to".to_owned(),
                url: build_rdd_url(platform, version_guid),
            }]
        }
        Platform::MacPlayer => {
            if version_guid.is_empty() {
                return Vec::new();
            }

            vec![
                DownloadLink {
                    name: "Apple Silicon".to_owned(),
                    url: format!("{SETUP_CDN_BASE_URL}/mac/arm64/{version_guid}-RobloxPlayer.zip"),
                },
                DownloadLink {
                    name: "Intel".to_owned(),
                    url: format!("{SETUP_CDN_BASE_URL}/mac/{version_guid}-RobloxPlayer.zip"),
                },
            ]
        }
        Platform::Android => {
            if deployment.version.is_empty() {
                return Vec::new();
            }

            vec![DownloadLink {
                name: "APKMirror".to_owned(),
                url: build_apk_mirror_url(&deployment.version),
            }]
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deployment(platform: Platform, version: &str, guid: &str) -> Deployment {
        Deployment {
            platform,
            channel: "LIVE".to_owned(),
            version: version.to_owned(),
            client_version_upload: guid.to_owned(),
        }
    }

    #[test]
    fn live_and_zlive_are_production_channels() {
        assert!(is_production_channel("LIVE"));
        assert!(is_production_channel("live"));
        assert!(is_production_channel("zLive"));
        assert!(is_production_channel("production"));
        assert!(!is_production_channel("zCanary"));
    }

    #[test]
    fn builds_client_settings_urls() {
        assert_eq!(
            client_settings_url(Platform::WindowsPlayer, "LIVE"),
            "https://clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer"
        );
        assert_eq!(
            client_settings_url(Platform::MacStudio, "zCanary"),
            "https://clientsettingscdn.roblox.com/v2/client-version/MacStudio/channel/zCanary"
        );
        assert_eq!(client_settings_url(Platform::Android, "LIVE"), "");
    }

    #[test]
    fn builds_windows_and_mac_download_links() {
        let windows = download_links(
            Platform::WindowsPlayer,
            &deployment(Platform::WindowsPlayer, "0.742.0", "version-abc"),
        );

        assert_eq!(windows.len(), 1);
        assert_eq!(
            windows[0].url,
            "https://rdd.latte.to/?binaryType=WindowsPlayer&channel=LIVE&version=version-abc"
        );

        let mac = download_links(
            Platform::MacPlayer,
            &deployment(Platform::MacPlayer, "0.742.0", "version-abc"),
        );

        assert_eq!(mac.len(), 2);
        assert_eq!(
            mac[0].url,
            "https://setup.rbxcdn.com/mac/arm64/version-abc-RobloxPlayer.zip"
        );
        assert_eq!(
            mac[1].url,
            "https://setup.rbxcdn.com/mac/version-abc-RobloxPlayer.zip"
        );
    }

    #[test]
    fn builds_android_download_link() {
        let android = download_links(
            Platform::Android,
            &deployment(Platform::Android, "2.642.783", ""),
        );

        assert_eq!(android.len(), 1);
        assert_eq!(
            android[0].url,
            "https://www.apkmirror.com/apk/roblox-corporation/roblox/roblox-2-642-783-release/"
        );
    }
}
