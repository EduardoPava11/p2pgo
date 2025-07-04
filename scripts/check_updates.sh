#!/bin/bash
# Check for updates using GitHub releases API

REPO="danielbank/p2pgo"
CURRENT_VERSION=$(cat VERSION)
API_URL="https://api.github.com/repos/$REPO/releases/latest"

echo "Current version: $CURRENT_VERSION"
echo "Checking for updates..."

# Get latest release from GitHub
LATEST_RELEASE=$(curl -s $API_URL)
LATEST_VERSION=$(echo $LATEST_RELEASE | jq -r '.tag_name' | sed 's/^v//')
DOWNLOAD_URL=$(echo $LATEST_RELEASE | jq -r '.assets[0].browser_download_url')

if [ "$LATEST_VERSION" = "null" ]; then
    echo "No releases found on GitHub"
    exit 0
fi

echo "Latest version: $LATEST_VERSION"

# Compare versions
if [ "$CURRENT_VERSION" = "$LATEST_VERSION" ]; then
    echo "You are running the latest version!"
else
    echo "Update available: $LATEST_VERSION"
    echo "Download URL: $DOWNLOAD_URL"
    echo ""
    echo "To update, download the latest release from:"
    echo "https://github.com/$REPO/releases/latest"
fi