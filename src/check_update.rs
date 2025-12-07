use notify_rust::Notification;
use reqwest;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tracing::error;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CachedRelease {
    pub tag_name: String,
    pub html_url: String,
    pub prerelease: bool,
    pub timestamp: u64,
}

#[derive(Deserialize, Debug)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    prerelease: bool,
}

pub fn check_for_updates(cache_dir: &PathBuf) -> Option<CachedRelease> {
    fs::create_dir_all(&cache_dir)
        .inspect_err(|e| error!("Error: {e}"))
        .ok()?;

    let cache_file = cache_dir.join("latest_release.json");
    let current_version = Version::parse(env!("CARGO_PKG_VERSION"))
        .inspect_err(|e| error!("Error: {e}"))
        .ok()?;

    let should_fetch = if cache_file.exists() {
        let cached_data: CachedRelease = serde_json::from_str(
            &fs::read_to_string(&cache_file)
                .inspect_err(|e| error!("Error: {e}"))
                .ok()?,
        )
        .inspect_err(|e| error!("Error: {e}"))
        .ok()?;
        let cached_time = SystemTime::UNIX_EPOCH + Duration::from_secs(cached_data.timestamp);
        let elapsed = SystemTime::now()
            .duration_since(cached_time)
            .inspect_err(|e| error!("Error: {e}"))
            .ok()?;

        elapsed.as_secs() > 172800
    } else {
        true
    };

    let latest_release = if should_fetch {
        let repo_path = env!("CARGO_PKG_REPOSITORY")
            .trim_start_matches("https://github.com/")
            .trim_end_matches(".git");

        let api_url = format!("https://api.github.com/repos/{}/releases", repo_path);

        let client = reqwest::blocking::Client::new();
        let releases: Vec<GithubRelease> = client
            .get(&api_url)
            .header("User-Agent", "rust-update-checker")
            .send()
            .inspect_err(|e| error!("Error: {e}"))
            .ok()?
            .json()
            .inspect_err(|e| error!("Error: {e}"))
            .ok()?;

        let latest = releases
            .first()
            .ok_or("No releases found")
            .inspect_err(|e| error!("Error: {e}"))
            .ok()?;

        let cached = CachedRelease {
            tag_name: latest.tag_name.clone(),
            html_url: latest.html_url.clone(),
            prerelease: latest.prerelease,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .inspect_err(|e| error!("Error: {e}"))
                .ok()?
                .as_secs(),
        };

        fs::write(
            &cache_file,
            serde_json::to_string_pretty(&cached)
                .inspect_err(|e| error!("Error: {e}"))
                .ok()?,
        )
        .inspect_err(|e| error!("Error: {e}"))
        .ok()?;

        cached
    } else {
        serde_json::from_str(
            &fs::read_to_string(&cache_file)
                .inspect_err(|e| error!("Error: {e}"))
                .ok()?,
        )
        .inspect_err(|e| error!("Error: {e}"))
        .ok()?
    };

    let release_version_str = latest_release.tag_name.trim_start_matches('v');
    let release_version = Version::parse(release_version_str)
        .inspect_err(|e| error!("Error: {e}"))
        .ok()?;

    if release_version > current_version {
        let _n = Notification::new()
            .summary("Actualización disponible")
            .body(&format!(
                "Nueva versión ({}) disponible",
                latest_release.tag_name,
            ))
            .show()
            .inspect_err(|e| error!("Error: {e}"))
            .ok()?;
        #[cfg(target_os = "linux")]
        _n.wait_for_action(|action| {
            if let "default" | "clicked" = action {
                let _ = open::that(&latest_release.html_url);
            }
        });

        return Some(latest_release);
    }

    None
}
