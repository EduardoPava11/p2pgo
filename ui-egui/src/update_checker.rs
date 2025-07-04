// SPDX-License-Identifier: MIT OR Apache-2.0

//! Update checking mechanism for P2P Go app
//! 
//! This module provides functionality to check for updates from a local file or URL,
//! parse update manifests, and determine if updates are available.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::Path;
use anyhow::{Result, Context};

/// Update manifest structure that describes available updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// Latest version available
    pub latest_version: Version,
    
    /// Minimum required version (forces update if current version is below this)
    pub minimum_version: Option<Version>,
    
    /// Update channels (stable, beta, etc.)
    #[serde(default)]
    pub channels: Vec<UpdateChannel>,
    
    /// Global announcement message (e.g., for maintenance)
    pub announcement: Option<String>,
    
    /// Manifest schema version for future compatibility
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 {
    1
}

/// Semantic version representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    /// Optional pre-release tag (e.g., "beta.1", "rc.2")
    pub pre_release: Option<String>,
}

impl Version {
    /// Parse version from string (e.g., "1.2.3" or "1.2.3-beta.1")
    pub fn parse(version_str: &str) -> Result<Self> {
        let version_str = version_str.trim();
        
        // Split into version and pre-release parts
        let (version_part, pre_release) = if let Some(dash_pos) = version_str.find('-') {
            let (ver, pre) = version_str.split_at(dash_pos);
            (ver, Some(pre[1..].to_string()))
        } else {
            (version_str, None)
        };
        
        // Parse version numbers
        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            anyhow::bail!("Invalid version format: expected major.minor.patch");
        }
        
        Ok(Version {
            major: parts[0].parse().context("Invalid major version")?,
            minor: parts[1].parse().context("Invalid minor version")?,
            patch: parts[2].parse().context("Invalid patch version")?,
            pre_release,
        })
    }
    
    /// Convert version to string representation
    pub fn to_string(&self) -> String {
        if let Some(ref pre) = self.pre_release {
            format!("{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
        } else {
            format!("{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare major, minor, patch
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            other => return other,
        }
        
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            other => return other,
        }
        
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            other => return other,
        }
        
        // Handle pre-release versions
        // No pre-release is greater than any pre-release
        match (&self.pre_release, &other.pre_release) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

/// Update channel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChannel {
    /// Channel name (e.g., "stable", "beta", "nightly")
    pub name: String,
    
    /// Latest version for this channel
    pub version: Version,
    
    /// Update details for this channel
    pub update: UpdateInfo,
}

/// Information about a specific update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// Download URL for the update
    pub download_url: String,
    
    /// SHA256 checksum of the download
    pub sha256: String,
    
    /// File size in bytes
    pub size: u64,
    
    /// Release notes or changelog
    pub release_notes: String,
    
    /// Release date (ISO 8601 format)
    pub release_date: String,
    
    /// Whether this update can be applied in-place
    pub supports_in_place: bool,
    
    /// Platform-specific information
    #[serde(default)]
    pub platforms: Vec<PlatformInfo>,
}

/// Platform-specific update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Platform identifier (e.g., "macos", "windows", "linux")
    pub platform: String,
    
    /// Architecture (e.g., "x86_64", "aarch64")
    pub arch: String,
    
    /// Platform-specific download URL (overrides main download_url)
    pub download_url: Option<String>,
    
    /// Platform-specific SHA256 (overrides main sha256)
    pub sha256: Option<String>,
    
    /// Additional platform-specific notes
    pub notes: Option<String>,
}

/// Result of an update check
#[derive(Debug, Clone)]
pub struct UpdateCheckResult {
    /// Current version of the app
    pub current_version: Version,
    
    /// Whether an update is available
    pub update_available: bool,
    
    /// Whether the update is mandatory (current version < minimum_version)
    pub update_required: bool,
    
    /// Available update info if update is available
    pub update_info: Option<UpdateInfo>,
    
    /// Latest version available
    pub latest_version: Option<Version>,
    
    /// Announcement message if any
    pub announcement: Option<String>,
}

/// Update checker that handles checking for updates
pub struct UpdateChecker {
    /// Current version of the application
    current_version: Version,
    
    /// Update channel to use (e.g., "stable", "beta")
    channel: String,
    
    /// HTTP client for fetching remote manifests
    #[cfg(not(test))]
    client: reqwest::Client,
}

impl UpdateChecker {
    /// Create a new update checker
    pub fn new(current_version: Version, channel: String) -> Self {
        Self {
            current_version,
            channel,
            #[cfg(not(test))]
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }
    
    /// Check for updates from a manifest URL
    pub async fn check_url(&self, url: &str) -> Result<UpdateCheckResult> {
        #[cfg(not(test))]
        {
            let response = self.client.get(url)
                .send()
                .await
                .context("Failed to fetch update manifest")?;
            
            let manifest_text = response.text()
                .await
                .context("Failed to read manifest response")?;
            
            let manifest: UpdateManifest = serde_json::from_str(&manifest_text)
                .context("Failed to parse update manifest")?;
            
            self.check_manifest(manifest)
        }
        
        #[cfg(test)]
        {
            // Test implementation
            let _ = url;
            Ok(UpdateCheckResult {
                current_version: self.current_version.clone(),
                update_available: false,
                update_required: false,
                update_info: None,
                latest_version: None,
                announcement: None,
            })
        }
    }
    
    /// Check for updates from a local manifest file
    pub fn check_file(&self, path: &Path) -> Result<UpdateCheckResult> {
        let manifest_text = std::fs::read_to_string(path)
            .context("Failed to read manifest file")?;
        
        let manifest: UpdateManifest = serde_json::from_str(&manifest_text)
            .context("Failed to parse update manifest")?;
        
        self.check_manifest(manifest)
    }
    
    /// Check for updates using a provided manifest
    pub fn check_manifest(&self, manifest: UpdateManifest) -> Result<UpdateCheckResult> {
        // Check if current version is below minimum required
        let update_required = if let Some(ref min_version) = manifest.minimum_version {
            self.current_version < *min_version
        } else {
            false
        };
        
        // Find the appropriate channel or use latest_version
        let (latest_version, update_info) = if self.channel == "stable" || manifest.channels.is_empty() {
            // Use the main latest_version for stable or if no channels defined
            let update_available = self.current_version < manifest.latest_version;
            
            // Create a basic UpdateInfo if none exists in channels
            let info = if update_available {
                Some(UpdateInfo {
                    download_url: String::new(), // Would be filled from channel
                    sha256: String::new(),
                    size: 0,
                    release_notes: String::new(),
                    release_date: String::new(),
                    supports_in_place: false,
                    platforms: vec![],
                })
            } else {
                None
            };
            
            (manifest.latest_version.clone(), info)
        } else {
            // Find the specific channel
            if let Some(channel) = manifest.channels.iter().find(|c| c.name == self.channel) {
                let update_available = self.current_version < channel.version;
                let info = if update_available {
                    Some(channel.update.clone())
                } else {
                    None
                };
                (channel.version.clone(), info)
            } else {
                // Channel not found, fall back to latest_version
                let update_available = self.current_version < manifest.latest_version;
                let info = if update_available {
                    Some(UpdateInfo {
                        download_url: String::new(),
                        sha256: String::new(),
                        size: 0,
                        release_notes: String::new(),
                        release_date: String::new(),
                        supports_in_place: false,
                        platforms: vec![],
                    })
                } else {
                    None
                };
                (manifest.latest_version.clone(), info)
            }
        };
        
        let update_available = self.current_version < latest_version;
        
        Ok(UpdateCheckResult {
            current_version: self.current_version.clone(),
            update_available,
            update_required,
            update_info,
            latest_version: Some(latest_version),
            announcement: manifest.announcement,
        })
    }
    
    /// Get platform-specific update info
    pub fn get_platform_info<'a>(&self, update_info: &'a UpdateInfo) -> Option<&'a PlatformInfo> {
        let platform = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        
        update_info.platforms.iter()
            .find(|p| p.platform == platform && p.arch == arch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_parsing() {
        let v1 = Version::parse("1.2.3").unwrap();
        assert_eq!(v1.major, 1);
        assert_eq!(v1.minor, 2);
        assert_eq!(v1.patch, 3);
        assert_eq!(v1.pre_release, None);
        
        let v2 = Version::parse("2.0.0-beta.1").unwrap();
        assert_eq!(v2.major, 2);
        assert_eq!(v2.minor, 0);
        assert_eq!(v2.patch, 0);
        assert_eq!(v2.pre_release, Some("beta.1".to_string()));
    }
    
    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.0.0").unwrap();
        let v2 = Version::parse("1.0.1").unwrap();
        let v3 = Version::parse("1.1.0").unwrap();
        let v4 = Version::parse("2.0.0").unwrap();
        let v5 = Version::parse("1.0.0-beta.1").unwrap();
        
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v5 < v1); // Pre-release is less than release
    }
    
    #[test]
    fn test_update_check() {
        let current = Version::parse("1.0.0").unwrap();
        let checker = UpdateChecker::new(current, "stable".to_string());
        
        let manifest = UpdateManifest {
            latest_version: Version::parse("1.1.0").unwrap(),
            minimum_version: None,
            channels: vec![],
            announcement: Some("New features available!".to_string()),
            schema_version: 1,
        };
        
        let result = checker.check_manifest(manifest).unwrap();
        assert!(result.update_available);
        assert!(!result.update_required);
        assert_eq!(result.announcement, Some("New features available!".to_string()));
    }
    
    #[test]
    fn test_required_update() {
        let current = Version::parse("1.0.0").unwrap();
        let checker = UpdateChecker::new(current, "stable".to_string());
        
        let manifest = UpdateManifest {
            latest_version: Version::parse("2.0.0").unwrap(),
            minimum_version: Some(Version::parse("1.5.0").unwrap()),
            channels: vec![],
            announcement: None,
            schema_version: 1,
        };
        
        let result = checker.check_manifest(manifest).unwrap();
        assert!(result.update_available);
        assert!(result.update_required);
    }
}