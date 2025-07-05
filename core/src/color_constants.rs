// SPDX-License-Identifier: MIT OR Apache-2.0

//! Color constants for the UI, including colorblind-safe palettes

/// Okabe-Ito colorblind-safe palette as RGB [0.0-1.0] values
/// Source: https://jfly.uni-koeln.de/color/
pub mod okabe_ito {
    pub const BLACK: [f32; 3] = [0.0, 0.0, 0.0]; // #000000
    pub const ORANGE: [f32; 3] = [0.9, 0.6, 0.0]; // #E69F00
    pub const LIGHT_BLUE: [f32; 3] = [0.35, 0.7, 0.9]; // #56B4E9
    pub const GREEN: [f32; 3] = [0.0, 0.6, 0.5]; // #009E73
    pub const YELLOW: [f32; 3] = [0.95, 0.9, 0.25]; // #F0E442
    pub const BLUE: [f32; 3] = [0.0, 0.45, 0.7]; // #0072B2
    pub const VERMILLION: [f32; 3] = [0.8, 0.4, 0.0]; // #D55E00
    pub const PURPLE: [f32; 3] = [0.8, 0.6, 0.7]; // #CC79A7

    // Additional useful colors that complement the palette
    pub const WHITE: [f32; 3] = [1.0, 1.0, 1.0]; // #FFFFFF
    pub const GRAY: [f32; 3] = [0.5, 0.5, 0.5]; // #808080
    pub const LIGHT_GRAY: [f32; 3] = [0.8, 0.8, 0.8]; // #CCCCCC
    pub const DARK_GRAY: [f32; 3] = [0.2, 0.2, 0.2]; // #333333
}

/// Okabe-Ito colorblind-safe palette as RGB 8-bit values [0-255]
pub mod okabe_ito_rgb {
    pub const BLACK: [u8; 3] = [0, 0, 0];
    pub const ORANGE: [u8; 3] = [230, 159, 0];
    pub const LIGHT_BLUE: [u8; 3] = [86, 180, 233];
    pub const GREEN: [u8; 3] = [0, 158, 115];
    pub const YELLOW: [u8; 3] = [240, 228, 66];
    pub const BLUE: [u8; 3] = [0, 114, 178];
    pub const VERMILLION: [u8; 3] = [213, 94, 0];
    pub const PURPLE: [u8; 3] = [204, 121, 167];

    // Additional colors
    pub const WHITE: [u8; 3] = [255, 255, 255];
    pub const GRAY: [u8; 3] = [128, 128, 128];
    pub const LIGHT_GRAY: [u8; 3] = [204, 204, 204];
    pub const DARK_GRAY: [u8; 3] = [51, 51, 51];
}

/// Okabe-Ito colorblind-safe palette as hex strings
pub mod okabe_ito_hex {
    pub const BLACK: &str = "#000000";
    pub const ORANGE: &str = "#E69F00";
    pub const LIGHT_BLUE: &str = "#56B4E9";
    pub const GREEN: &str = "#009E73";
    pub const YELLOW: &str = "#F0E442";
    pub const BLUE: &str = "#0072B2";
    pub const VERMILLION: &str = "#D55E00";
    pub const PURPLE: &str = "#CC79A7";

    // Additional colors
    pub const WHITE: &str = "#FFFFFF";
    pub const GRAY: &str = "#808080";
    pub const LIGHT_GRAY: &str = "#CCCCCC";
    pub const DARK_GRAY: &str = "#333333";
}

/// Relay status colors (colorblind-safe)
pub mod relay_status {
    use super::okabe_ito;

    pub const HEALTHY: [f32; 3] = okabe_ito::GREEN; // Good latency (<80ms)
    pub const DEGRADED: [f32; 3] = okabe_ito::ORANGE; // Higher latency (80-200ms)
    pub const WARNING: [f32; 3] = okabe_ito::YELLOW; // Potential issues
    pub const ERROR: [f32; 3] = okabe_ito::VERMILLION; // Connection errors
    pub const OFFLINE: [f32; 3] = okabe_ito::GRAY; // Not connected
    pub const RESTARTING: [f32; 3] = okabe_ito::LIGHT_BLUE; // Restarting state
}

/// Network status colors (for UI elements)
pub mod network_status {
    use super::okabe_ito;

    pub const OFFLINE: [f32; 3] = okabe_ito::GRAY; // No connection
    pub const CONNECTING: [f32; 3] = okabe_ito::LIGHT_BLUE; // Attempting to connect
    pub const CONNECTED: [f32; 3] = okabe_ito::GREEN; // Connected successfully
    pub const WARNING: [f32; 3] = okabe_ito::ORANGE; // Connected with issues
    pub const ERROR: [f32; 3] = okabe_ito::VERMILLION; // Connection error
    pub const SYNCING: [f32; 3] = okabe_ito::YELLOW; // Syncing data
}

/// Player colors
pub mod player {
    use super::okabe_ito;

    pub const BLACK: [f32; 3] = okabe_ito::BLACK; // Player 1 (Black)
    pub const WHITE: [f32; 3] = okabe_ito::WHITE; // Player 2 (White)
    pub const SPECTATOR: [f32; 3] = okabe_ito::LIGHT_BLUE; // Observer
}

/// Convert a color from [0.0-1.0] to [0-255] range
pub fn f32_to_u8_rgb(color: [f32; 3]) -> [u8; 3] {
    [
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
    ]
}

/// Convert a color from [0-255] to [0.0-1.0] range
pub fn u8_to_f32_rgb(color: [u8; 3]) -> [f32; 3] {
    [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
    ]
}

/// Convert RGB color (0.0-1.0) to RGBA with alpha component
pub fn rgb_to_rgba(color: [f32; 3], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], alpha]
}

/// Get a color with modified brightness (factor: 0.0 = black, 1.0 = original, >1.0 brighter)
pub fn adjust_brightness(color: [f32; 3], factor: f32) -> [f32; 3] {
    [
        (color[0] * factor).clamp(0.0, 1.0),
        (color[1] * factor).clamp(0.0, 1.0),
        (color[2] * factor).clamp(0.0, 1.0),
    ]
}

/// Create a gradient between two colors
pub fn linear_gradient(color1: [f32; 3], color2: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        color1[0] * (1.0 - t) + color2[0] * t,
        color1[1] * (1.0 - t) + color2[1] * t,
        color1[2] * (1.0 - t) + color2[2] * t,
    ]
}

/// Convert a hex color string (#RRGGBB) to RGB [0.0-1.0]
pub fn hex_to_rgb(hex: &str) -> Result<[f32; 3], String> {
    if hex.len() != 7 || !hex.starts_with('#') {
        return Err(format!("Invalid hex color: {}", hex));
    }

    let r = u8::from_str_radix(&hex[1..3], 16)
        .map_err(|_| format!("Invalid red component: {}", &hex[1..3]))?;

    let g = u8::from_str_radix(&hex[3..5], 16)
        .map_err(|_| format!("Invalid green component: {}", &hex[3..5]))?;

    let b = u8::from_str_radix(&hex[5..7], 16)
        .map_err(|_| format!("Invalid blue component: {}", &hex[5..7]))?;

    Ok([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_conversions() {
        let color_f32 = [0.5, 0.25, 0.75];
        let color_u8 = f32_to_u8_rgb(color_f32);

        assert_eq!(color_u8, [127, 63, 191]);

        let converted_back = u8_to_f32_rgb(color_u8);
        // Allow for small rounding errors in the conversion
        assert!((converted_back[0] - color_f32[0]).abs() < 0.01);
        assert!((converted_back[1] - color_f32[1]).abs() < 0.01);
        assert!((converted_back[2] - color_f32[2]).abs() < 0.01);
    }

    #[test]
    fn test_hex_to_rgb() {
        // Test valid hex colors
        assert_eq!(hex_to_rgb("#000000").unwrap(), [0.0, 0.0, 0.0]);
        assert_eq!(hex_to_rgb("#FFFFFF").unwrap(), [1.0, 1.0, 1.0]);
        assert_eq!(hex_to_rgb("#FF0000").unwrap(), [1.0, 0.0, 0.0]);

        // Test errors
        assert!(hex_to_rgb("invalid").is_err());
        assert!(hex_to_rgb("#12345").is_err());
        assert!(hex_to_rgb("#GGHHII").is_err());
    }
}
