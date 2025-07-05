#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Auto-update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Update check URL
    pub update_url: String,
    /// Current version
    pub current_version: semver::Version,
    /// Enable auto-update
    pub enabled: bool,
    /// Update channel (stable, beta, nightly)
    pub channel: UpdateChannel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UpdateChannel {
    Stable,
    Beta,
    Nightly,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            update_url: "https://api.github.com/repos/p2pgo/p2pgo/releases".to_string(),
            current_version: semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            enabled: true,
            channel: UpdateChannel::Stable,
        }
    }
}

/// Update manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: semver::Version,
    pub release_notes: String,
    pub download_url: String,
    pub signature: String,
    pub size: u64,
    pub sha256: String,
}

/// Auto-updater
pub struct AutoUpdater {
    config: UpdateConfig,
    http_client: reqwest::Client,
}

impl AutoUpdater {
    pub fn new(config: UpdateConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Check for updates
    pub async fn check_for_updates(&self) -> Result<Option<UpdateManifest>> {
        if !self.config.enabled {
            return Ok(None);
        }

        // Fetch latest release info
        let response = self
            .http_client
            .get(&self.config.update_url)
            .header("User-Agent", "P2PGo-Updater")
            .send()
            .await
            .context("Failed to fetch updates")?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let releases: Vec<GitHubRelease> = response.json().await?;

        // Find latest release for our channel
        for release in releases {
            if self.should_update(&release) {
                return Ok(Some(self.create_manifest(release)?));
            }
        }

        Ok(None)
    }

    /// Download and apply update
    pub async fn download_update(&self, manifest: &UpdateManifest) -> Result<PathBuf> {
        let temp_dir = std::env::temp_dir();
        let file_name = format!("p2pgo-update-{}.dmg", manifest.version);
        let download_path = temp_dir.join(&file_name);

        // Download update
        let response = self.http_client.get(&manifest.download_url).send().await?;

        let bytes = response.bytes().await?;

        // Verify checksum
        let hash = blake3::hash(&bytes);
        if hash.to_hex().as_str() != manifest.sha256 {
            anyhow::bail!("Update checksum mismatch");
        }

        // Save to disk
        std::fs::write(&download_path, bytes)?;

        Ok(download_path)
    }

    /// Apply downloaded update (platform-specific)
    #[cfg(target_os = "macos")]
    pub async fn apply_update(&self, dmg_path: PathBuf) -> Result<()> {
        use std::process::Command;

        // Mount DMG
        Command::new("hdiutil")
            .args(&["attach", dmg_path.to_str().unwrap()])
            .output()?;

        // Copy new app to Applications
        // This will prompt user for confirmation
        Command::new("open").arg(dmg_path).spawn()?;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub async fn apply_update(&self, _update_path: PathBuf) -> Result<()> {
        anyhow::bail!("Auto-update not supported on this platform")
    }

    fn should_update(&self, release: &GitHubRelease) -> bool {
        // Parse version from tag
        if let Ok(version) = semver::Version::parse(&release.tag_name.trim_start_matches('v')) {
            // Check if newer than current
            if version > self.config.current_version {
                // Check channel
                match self.config.channel {
                    UpdateChannel::Stable => !release.prerelease,
                    UpdateChannel::Beta => true,
                    UpdateChannel::Nightly => true,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn create_manifest(&self, release: GitHubRelease) -> Result<UpdateManifest> {
        // Find DMG asset
        let dmg_asset = release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".dmg"))
            .context("No DMG found in release")?;

        Ok(UpdateManifest {
            version: semver::Version::parse(&release.tag_name.trim_start_matches('v'))?,
            release_notes: release.body.unwrap_or_default(),
            download_url: dmg_asset.browser_download_url.clone(),
            signature: String::new(), // TODO: Implement signing
            size: dmg_asset.size,
            sha256: String::new(), // TODO: Add to release notes
        })
    }
}

/// GitHub release structure
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[allow(dead_code)]
    name: Option<String>,
    body: Option<String>,
    prerelease: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}
