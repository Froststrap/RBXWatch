use std::{sync::LazyLock, time::Duration};

use chrono::{DateTime, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use regex::Regex;

use super::{Error, Platform, links::SETUP_CDN_BASE_URL};

const DEPLOY_HISTORY_TAIL_BYTES: u64 = 65_536;
const DEPLOY_HISTORY_MAX_BYTES: usize = 4 << 20;
const DEPLOY_HISTORY_TIME_LAYOUT: &str = "%m/%d/%Y %I:%M:%S %p";
const DEPLOY_HISTORY_TIME_ZONE: Tz = Tz::America__Los_Angeles;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

static DEPLOY_HISTORY_LINE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^New (\S+) \S+ at (\d{1,2}/\d{1,2}/\d{4} \d{1,2}:\d{2}:\d{2} [AP]M),.*git hash: (\S+)",
    )
    .expect("valid deploy history pattern")
});

#[derive(Clone, Debug)]
pub struct DeployHistoryProvider {
    http: reqwest::Client,
}

impl Default for DeployHistoryProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DeployHistoryProvider {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    pub async fn get_build_time(
        &self,
        platform: Platform,
        version: &str,
    ) -> Result<DateTime<Utc>, Error> {
        let Some((endpoint, entry_kind)) = deploy_history_source(platform) else {
            return Err(Error::BuildTimeNotFound);
        };

        let mut response = self
            .http
            .get(endpoint)
            .header("Range", format!("bytes=-{DEPLOY_HISTORY_TAIL_BYTES}"))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;

        let status = response.status();

        if status != reqwest::StatusCode::OK && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(Error::HttpStatus(status.as_u16()));
        }

        let mut body = Vec::new();

        while let Some(chunk) = response.chunk().await? {
            let remaining = DEPLOY_HISTORY_MAX_BYTES.saturating_sub(body.len());

            if remaining == 0 {
                break;
            }

            body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        }

        let history = String::from_utf8_lossy(&body);

        find_build_time(&history, entry_kind, version)
    }
}

fn deploy_history_source(platform: Platform) -> Option<(String, &'static str)> {
    match platform {
        Platform::WindowsPlayer => Some((
            format!("{SETUP_CDN_BASE_URL}/DeployHistory.txt"),
            "WindowsPlayer",
        )),
        Platform::MacPlayer => Some((
            format!("{SETUP_CDN_BASE_URL}/mac/DeployHistory.txt"),
            "Client",
        )),
        _ => None,
    }
}

fn find_build_time(history: &str, entry_kind: &str, version: &str) -> Result<DateTime<Utc>, Error> {
    let mut found = None;

    for line in history.lines() {
        let Some(captures) = DEPLOY_HISTORY_LINE_PATTERN.captures(line.trim()) else {
            continue;
        };

        if &captures[1] != entry_kind || &captures[3] != version {
            continue;
        }

        let Ok(naive) = NaiveDateTime::parse_from_str(&captures[2], DEPLOY_HISTORY_TIME_LAYOUT)
        else {
            continue;
        };

        let local = match DEPLOY_HISTORY_TIME_ZONE.from_local_datetime(&naive) {
            LocalResult::Single(local) => local,
            LocalResult::Ambiguous(first, _) => first,
            LocalResult::None => continue,
        };

        // Later lines win, so a redeploy of the same version is used.
        found = Some(local.with_timezone(&Utc));
    }

    found.ok_or(Error::BuildTimeNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HISTORY: &str = concat!(
        "New WindowsPlayer version-hidden at 10/5/2026 3:38:32 PM, ",
        "file version: 0.742.0.7421053, git hash: 0.742.0.7421053 Done!\n",
        "New WindowsPlayer version-hidden at 10/6/2026 1:04:05 AM, ",
        "file version: 0.743.0.7431053, git hash: 0.743.0.7431053 Done!\n",
    );

    #[test]
    fn parses_build_time_in_pacific_time() {
        let built_at = find_build_time(HISTORY, "WindowsPlayer", "0.742.0.7421053")
            .expect("build time should be found");

        assert_eq!(
            built_at,
            Utc.with_ymd_and_hms(2026, 10, 5, 22, 38, 32).unwrap()
        );
    }

    #[test]
    fn uses_the_last_matching_line() {
        let history = format!(
            "{HISTORY}New WindowsPlayer version-hidden at 10/7/2026 5:00:00 PM, \
             file version: 0.742.0.7421053, git hash: 0.742.0.7421053 Done!\n"
        );

        let built_at = find_build_time(&history, "WindowsPlayer", "0.742.0.7421053")
            .expect("build time should be found");

        assert_eq!(
            built_at,
            Utc.with_ymd_and_hms(2026, 10, 8, 0, 0, 0).unwrap()
        );
    }

    #[test]
    fn ignores_other_platforms() {
        let err = find_build_time(HISTORY, "Client", "0.742.0.7421053")
            .expect_err("macOS entry should not match");

        assert!(matches!(err, Error::BuildTimeNotFound));
    }

    #[test]
    fn returns_not_found_for_missing_version() {
        let err = find_build_time(HISTORY, "WindowsPlayer", "0.700.0.7000000")
            .expect_err("unknown version should not match");

        assert!(matches!(err, Error::BuildTimeNotFound));
    }
}
