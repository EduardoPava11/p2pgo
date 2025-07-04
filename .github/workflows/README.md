# GitHub Actions Workflows

## Overview

This directory contains GitHub Actions workflows for CI/CD.

## Workflows

### ci-simple.yml
- **Purpose**: Basic CI checks that work reliably
- **Triggers**: Push to main, PRs
- **Jobs**: 
  - Test core crate on macOS
  - Build check on Ubuntu
  - Formatting and clippy

### deploy-pages.yml
- **Purpose**: Deploy website to GitHub Pages
- **Triggers**: Push to main
- **Permissions**: Requires pages write and id-token write
- **Deploys**: docs/ folder to GitHub Pages

### release.yml
- **Purpose**: Build and release macOS DMG
- **Triggers**: Git tags (v*)
- **Jobs**: Build universal DMG and create GitHub release

### ci.yml (original)
- **Status**: May have issues with Linux dependencies
- **Note**: Keep for reference, but ci-simple.yml is more reliable

## Common Issues

### Linux Build Failures
Many Linux builds fail due to missing system dependencies:
- glib-2.0-dev
- gtk-3-dev
- webkit2gtk-4.0-dev

### Pages Deployment
Requires proper permissions in workflow:
```yaml
permissions:
  contents: read
  pages: write
  id-token: write
```

## Local Testing

Test workflows locally with [act](https://github.com/nektos/act):
```bash
act -W .github/workflows/ci-simple.yml
```