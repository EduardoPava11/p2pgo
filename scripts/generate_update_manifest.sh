#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Script to generate an update manifest for P2P Go
# Usage: ./generate_update_manifest.sh <version> [channel]

set -e

VERSION=${1:-"0.2.0"}
CHANNEL=${2:-"stable"}
RELEASE_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Parse version components
IFS='.' read -r MAJOR MINOR PATCH <<< "$VERSION"
PRE_RELEASE=""
if [[ $VERSION == *"-"* ]]; then
    PRE_RELEASE=$(echo $VERSION | cut -d'-' -f2)
    IFS='.' read -r MAJOR MINOR PATCH <<< $(echo $VERSION | cut -d'-' -f1)
fi

# Generate release notes (in real usage, this would come from CHANGELOG or git)
RELEASE_NOTES="## What's New in v${VERSION}

- Improved network connectivity
- Added support for larger board sizes  
- Fixed scoring edge cases
- Performance improvements
- Better error handling

## Bug Fixes

- Fixed crash when disconnecting during game
- Resolved UI freezing issues
- Corrected stone capture logic"

# Create the manifest
cat > update_manifest.json << EOF
{
  "latest_version": {
    "major": ${MAJOR},
    "minor": ${MINOR},
    "patch": ${PATCH},
    "pre_release": $([ -z "$PRE_RELEASE" ] && echo "null" || echo "\"$PRE_RELEASE\"")
  },
  "minimum_version": null,
  "channels": [
    {
      "name": "${CHANNEL}",
      "version": {
        "major": ${MAJOR},
        "minor": ${MINOR},
        "patch": ${PATCH},
        "pre_release": $([ -z "$PRE_RELEASE" ] && echo "null" || echo "\"$PRE_RELEASE\"")
      },
      "update": {
        "download_url": "https://github.com/example/p2pgo/releases/download/v${VERSION}/P2P-Go-${VERSION}.dmg",
        "sha256": "$(echo -n "placeholder-sha256-for-${VERSION}" | shasum -a 256 | cut -d' ' -f1)",
        "size": 52428800,
        "release_notes": $(echo "$RELEASE_NOTES" | jq -Rs .),
        "release_date": "${RELEASE_DATE}",
        "supports_in_place": false,
        "platforms": [
          {
            "platform": "macos",
            "arch": "x86_64", 
            "download_url": "https://github.com/example/p2pgo/releases/download/v${VERSION}/P2P-Go-${VERSION}-x64.dmg",
            "sha256": "$(echo -n "placeholder-sha256-x64-for-${VERSION}" | shasum -a 256 | cut -d' ' -f1)",
            "notes": "Requires macOS 10.15 or later"
          },
          {
            "platform": "macos",
            "arch": "aarch64",
            "download_url": "https://github.com/example/p2pgo/releases/download/v${VERSION}/P2P-Go-${VERSION}-arm64.dmg",
            "sha256": "$(echo -n "placeholder-sha256-arm64-for-${VERSION}" | shasum -a 256 | cut -d' ' -f1)",
            "notes": "Native Apple Silicon support"
          }
        ]
      }
    }
  ],
  "announcement": null,
  "schema_version": 1
}
EOF

echo "Generated update_manifest.json for version ${VERSION} (${CHANNEL} channel)"
echo ""
echo "Next steps:"
echo "1. Update the download URLs to point to actual release files"
echo "2. Calculate real SHA256 checksums for the release files"
echo "3. Update file sizes with actual values"
echo "4. Customize release notes as needed"
echo "5. Host the manifest at a stable URL for the app to check"