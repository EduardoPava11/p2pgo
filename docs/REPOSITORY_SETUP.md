# P2P Go Repository Setup Guide

## Initial Setup

### 1. Create GitHub Repository

1. Go to https://github.com/new
2. Repository name: `p2pgo`
3. Description: "Decentralized Go game with neural network AI - no servers required!"
4. Make it public
5. Don't initialize with README (we have one)
6. Create repository

### 2. Push Local Code

```bash
# In your p2pgo directory
git init
git add .
git commit -m "Initial commit: P2P Go with neural network AI"
git branch -M main
git remote add origin https://github.com/YOUR_USERNAME/p2pgo.git
git push -u origin main
```

### 3. Enable GitHub Pages

1. Go to Settings → Pages
2. Source: Deploy from a branch
3. Branch: main
4. Folder: /docs
5. Save

Your website will be available at: https://YOUR_USERNAME.github.io/p2pgo/

### 4. Create First Release

1. Go to Releases → Create a new release
2. Choose a tag: `v1.0.0`
3. Release title: "P2P Go v1.0.0 - Initial Release"
4. Upload the DMG file: `P2PGo-universal.dmg`
5. Description:
```markdown
## 🎉 First Release!

P2P Go is now available for macOS! Play Go directly with friends over peer-to-peer connections, enhanced by neural network AI.

### Features
- ✅ Direct P2P connections (no servers!)
- ✅ Neural network move suggestions
- ✅ Clean, modern interface
- ✅ SGF import/export
- ✅ Universal binary (Intel & Apple Silicon)

### Installation
1. Download `P2PGo-universal.dmg`
2. Open and drag to Applications
3. Launch and create a game!

### Known Issues
- The game creator needs to wait in the game view (not lobby) for opponents
- Windows and Linux versions coming soon

Report issues: https://github.com/YOUR_USERNAME/p2pgo/issues
```

### 5. Update Website Links

Edit these files to update YOUR_USERNAME:
- `docs/index.html` - Replace all instances of `yourusername`
- `docs/js/main.js` - Update GitHub API URLs
- `README.md` - Update all GitHub links

### 6. Add Screenshots

Create these screenshots and save to `docs/images/`:
1. `hero-screenshot.png` - Main game view with neural panel
2. `gameplay.png` - Active game in progress
3. `icon.png` - App icon (512x512)

### 7. Configure Branch Protection

1. Go to Settings → Branches
2. Add rule for `main`
3. Enable:
   - Require pull request reviews
   - Dismiss stale reviews
   - Require status checks (CI)
   - Require branches to be up to date

### 8. Set Up Project Board

1. Go to Projects → New project
2. Choose "Board" template
3. Columns: Backlog, In Progress, Review, Done
4. Add initial issues from the roadmap

### 9. Create Issue Templates

Create `.github/ISSUE_TEMPLATE/`:
- `bug_report.md`
- `feature_request.md`
- `enhancement.md`

### 10. Community Files

Add:
- `CONTRIBUTING.md` - Contribution guidelines
- `CODE_OF_CONDUCT.md` - Community standards
- `SECURITY.md` - Security policy

## Maintenance

### Regular Updates

1. **Weekly**: Review and triage issues
2. **Monthly**: Update dependencies
3. **Quarterly**: Review roadmap progress

### Release Process

1. Update version in `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Create and push tag: `git tag -a v1.1.0 -m "Release v1.1.0"`
4. GitHub Actions will automatically build and create release

### Documentation

Keep these updated:
- README.md - Main documentation
- Architecture specs in `docs/`
- API documentation (when added)
- Website content

## Promotion

### Where to Share

1. **Hacker News**: Focus on P2P and no-server aspects
2. **Reddit**: r/baduk, r/rust, r/p2p
3. **Go Forums**: lifein19x19.com, Online Go Server forums
4. **Dev Communities**: dev.to, hashnode
5. **Social Media**: Twitter/X with #golang #p2p #rust tags

### Key Messages

- "Play Go without servers or accounts"
- "Neural network AI that runs locally"
- "True peer-to-peer, your games stay private"
- "Open source and free forever"

## Analytics (Optional)

If you want basic analytics without compromising privacy:
- Use Plausible or Umami (privacy-focused)
- Only track download counts
- No user tracking in the app itself

## Support

Set up support channels:
1. GitHub Discussions for community help
2. Issues for bugs and features
3. Optional: Discord server for real-time chat

Remember: The goal is a community-driven project that respects user privacy while providing an excellent Go experience!