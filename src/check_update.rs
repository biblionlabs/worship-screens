use notify_rust::Notification;
use reqwest;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Serialize, Deserialize, Debug)]
struct CachedRelease {
    tag_name: String,
    html_url: String,
    prerelease: bool,
    timestamp: u64,
}

#[derive(Deserialize, Debug)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    prerelease: bool,
}

pub fn check_for_updates(cache_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&cache_dir)?;

    let cache_file = cache_dir.join("latest_release.json");
    let current_version = Version::parse(env!("CARGO_PKG_VERSION"))?;

    let should_fetch = if cache_file.exists() {
        let cached_data: CachedRelease = serde_json::from_str(&fs::read_to_string(&cache_file)?)?;
        let cached_time = SystemTime::UNIX_EPOCH + Duration::from_secs(cached_data.timestamp);
        let elapsed = SystemTime::now().duration_since(cached_time)?;

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
            .send()?
            .json()?;

        let latest = releases.first().ok_or("No releases found")?;

        let cached = CachedRelease {
            tag_name: latest.tag_name.clone(),
            html_url: latest.html_url.clone(),
            prerelease: latest.prerelease,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs(),
        };

        fs::write(&cache_file, serde_json::to_string_pretty(&cached)?)?;

        cached
    } else {
        serde_json::from_str(&fs::read_to_string(&cache_file)?)?
    };

    let release_version_str = latest_release.tag_name.trim_start_matches('v');
    let release_version = Version::parse(release_version_str)?;

    if release_version > current_version {
        let _n = Notification::new()
            .summary("Actualización disponible")
            .body(&format!(
                "Nueva versión ({}) disponible",
                latest_release.tag_name,
            ))
            .show()?;
        #[cfg(target_os = "linux")]
        _n.wait_for_action(|action| {
            if let "default" | "clicked" = action {
                let _ = open::that(&latest_release.html_url);
            }
        });
    }

    Ok(())
}
