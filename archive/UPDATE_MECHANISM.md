# P2P Go Update Mechanism

This document describes the update checking mechanism implemented for the P2P Go application.

## Overview

The update checking system allows the app to:
- Check for updates from a local file or remote URL
- Display update notifications in the UI
- Support both in-place updates and reinstall flows
- Use a flexible manifest format for update information

## Components

### 1. Update Checker (`ui-egui/src/update_checker.rs`)

The core update checking logic:
- `UpdateChecker` - Main struct that performs update checks
- `UpdateManifest` - JSON structure describing available updates
- `Version` - Semantic versioning with pre-release support
- `UpdateCheckResult` - Result of an update check

### 2. Update UI (`ui-egui/src/update_ui.rs`)

UI components for displaying updates:
- `UpdateNotification` - Non-intrusive notification shown in top-right
- `UpdateDialog` - Progress dialog for update installation
- `UpdateAction` - User actions (update now, remind later, skip)

### 3. Integration in App

The update checker is integrated into the main app:
- Checks for updates periodically (default: every 12 hours)
- Shows notification when update is available
- Displays update indicator in main menu
- Handles user actions appropriately

## Update Manifest Format

The update manifest is a JSON file with the following structure:

```json
{
  "latest_version": {
    "major": 0,
    "minor": 2,
    "patch": 0,
    "pre_release": null
  },
  "minimum_version": {
    "major": 0,
    "minor": 1,
    "patch": 0,
    "pre_release": null
  },
  "channels": [
    {
      "name": "stable",
      "version": { ... },
      "update": {
        "download_url": "https://...",
        "sha256": "...",
        "size": 52428800,
        "release_notes": "...",
        "release_date": "2025-01-02T12:00:00Z",
        "supports_in_place": false,
        "platforms": [
          {
            "platform": "macos",
            "arch": "x86_64",
            "download_url": "...",
            "sha256": "...",
            "notes": "Requires macOS 10.15 or later"
          }
        ]
      }
    }
  ],
  "announcement": "Optional global message",
  "schema_version": 1
}
```

### Fields

- `latest_version`: The latest stable version available
- `minimum_version`: Optional minimum required version (forces update)
- `channels`: Different release channels (stable, beta, etc.)
- `announcement`: Optional message shown to all users
- `schema_version`: For future manifest format changes

## Usage

### Local File Checking

Place an `update_manifest.json` file in the app's working directory. The app will check it on startup and periodically.

### Remote URL Checking

Configure the update checker to fetch from a URL:

```rust
let update_url = "https://example.com/p2pgo/update_manifest.json";
let result = update_checker.check_url(update_url).await?;
```

### Testing

1. Create an `update_manifest.json` with a version higher than current
2. Run the app - it should show an update notification
3. Click the notification to see update details
4. Test different actions (update, remind later, skip)

## Future Enhancements

The current implementation provides the foundation for updates. Future work could include:

1. **Download & Install**: 
   - Download update files with progress tracking
   - Verify SHA256 checksums
   - Extract and install updates

2. **In-Place Updates**:
   - For minor updates, replace binary without full reinstall
   - Requires careful permission handling

3. **Delta Updates**:
   - Download only changed files
   - Reduce bandwidth usage

4. **Rollback Support**:
   - Keep previous version for rollback
   - Handle update failures gracefully

5. **Auto-Update Settings**:
   - User preferences for automatic updates
   - Background download options
   - Update scheduling

6. **Code Signing**:
   - Verify update authenticity
   - Prevent tampering

## Security Considerations

1. Always use HTTPS for update manifest URLs
2. Verify SHA256 checksums before installing
3. Consider code signing for distributed binaries
4. Validate manifest schema version
5. Handle network errors gracefully
6. Don't expose sensitive information in manifests

## Platform-Specific Notes

### macOS
- DMG files for distribution
- May require notarization for updates
- Handle Gatekeeper requirements

### Windows
- MSI or NSIS installers
- Consider Windows SmartScreen
- Handle UAC elevation for installs

### Linux
- AppImage for universal compatibility
- Respect package manager conventions
- Handle different distributions