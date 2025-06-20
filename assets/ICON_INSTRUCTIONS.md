# App Icon Creation Instructions

To create the p2pgo app icon, follow these steps:

## Required Icons for macOS App Bundle

Create the following icon sizes in PNG format:

- icon_16x16.png
- icon_16x16@2x.png (32x32)
- icon_32x32.png  
- icon_32x32@2x.png (64x64)
- icon_128x128.png
- icon_128x128@2x.png (256x256)
- icon_256x256.png
- icon_256x256@2x.png (512x512)
- icon_512x512.png
- icon_512x512@2x.png (1024x1024)

## Design Concept

The icon should represent:
- A Go board (19x19 grid)
- Network/P2P connectivity (interconnected nodes)
- Modern, clean design suitable for macOS

## Suggested Design Elements

1. **Base**: A simplified Go board with grid lines
2. **Stones**: A few black and white stones placed on the board
3. **Network**: Subtle network connection lines or nodes around the edges
4. **Colors**: Use colors that work well in both light and dark modes

## Tools for Creation

- Use a vector graphics tool like Sketch, Figma, or Adobe Illustrator
- Export at the required resolutions
- Consider using SF Symbols for consistency with macOS design

## Icon Bundle

Once created, place all icons in this assets/ directory and update the cargo-dist configuration to reference them.

## DMG Background (Optional)

For the DMG installer, you can also create:
- dmg-background.png (660x400 pixels)
- Should show the app icon and provide visual guidance for installation
