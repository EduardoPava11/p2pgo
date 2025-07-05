//! Example demonstrating UI customization with WASM tensors
//!
//! Run with: cargo run --example ui_customization -p p2pgo-ui-egui

use egui::Color32;
use p2pgo_ui_egui::ui_config::{SerializableColor, TerritoryMarkerType, UiConfig};

fn main() {
    println!("🎨 P2P Go UI Customization Examples\n");

    // Create default config
    let mut config = UiConfig::default();
    println!("📝 Default UI Configuration:");
    println!("   Window size: {:?}", config.window.initial_size);
    println!("   Board size: {}", config.board.size);
    println!(
        "   Grid color: RGB({},{},{})",
        config.board.grid_color.r, config.board.grid_color.g, config.board.grid_color.b
    );
    println!("   Grid line width: {}", config.board.grid_line_width);
    println!("   Stone radius ratio: {}", config.board.stone_radius_ratio);
    println!("   Territory marker: {:?}", config.territory.marker_type);

    // Apply WASM tensor parameters
    println!("\n🧠 Applying WASM Tensor Parameters:");

    // Simulate different tensor values
    let tensor_sets = vec![
        ("Minimal", vec![0.0, 0.0, 0.0]),
        ("Default", vec![0.5, 0.5, 0.5]),
        ("Bold", vec![1.0, 1.0, 1.0]),
        ("Mixed", vec![0.2, 0.8, 0.4]),
    ];

    for (name, tensors) in tensor_sets {
        println!("\n   {} tensors: {:?}", name, tensors);
        config.apply_tensor_params(&tensors);
        println!("      Grid line width: {:.2}", config.board.grid_line_width);
        println!(
            "      Stone radius ratio: {:.3}",
            config.board.stone_radius_ratio
        );
        println!(
            "      Territory marker ratio: {:.3}",
            config.territory.marker_size_ratio
        );
    }

    // Create custom theme
    println!("\n🎨 Creating Custom Themes:");

    // Dark theme
    let mut dark_config = UiConfig::default();
    dark_config.window.background_color = Color32::from_rgb(30, 30, 30).into();
    dark_config.board.background_color = Color32::from_rgb(50, 50, 50).into();
    dark_config.board.grid_color = Color32::from_rgb(200, 200, 200).into();
    dark_config.board.black_stone_color = Color32::from_rgb(10, 10, 10).into();
    dark_config.board.white_stone_color = Color32::from_rgb(230, 230, 230).into();
    dark_config.button.background_color = Color32::from_rgb(60, 60, 60).into();
    dark_config.button.text_color = Color32::WHITE.into();

    println!("   ⚫ Dark Theme created");

    // Nature theme
    let mut nature_config = UiConfig::default();
    nature_config.board.background_color = Color32::from_rgb(139, 90, 43).into(); // Saddle brown
    nature_config.board.grid_color = Color32::from_rgb(101, 67, 33).into(); // Dark brown
    nature_config.board.grid_line_width = 2.0;
    nature_config.territory.marker_type = TerritoryMarkerType::Circle;

    println!("   🌳 Nature Theme created");

    // Modern theme
    let mut modern_config = UiConfig::default();
    modern_config.board.background_color = Color32::from_rgb(240, 240, 240).into();
    modern_config.board.grid_color = Color32::from_rgb(100, 100, 100).into();
    modern_config.board.grid_line_width = 1.0;
    modern_config.button.corner_radius = 20.0;
    modern_config.button.shadow_offset = Some((4.0, 4.0));
    modern_config.territory.marker_type = TerritoryMarkerType::Overlay;

    println!("   💎 Modern Theme created");

    // Save and load config
    println!("\n💾 Saving Configuration:");
    let config_path = std::path::Path::new("example_ui_config.json");

    match modern_config.save_to_file(config_path) {
        Ok(_) => {
            println!("   ✅ Saved to example_ui_config.json");

            // Load it back
            match UiConfig::load_from_file(config_path) {
                Ok(loaded) => {
                    println!("   ✅ Loaded config successfully");
                    println!(
                        "      Button corner radius: {}",
                        loaded.button.corner_radius
                    );
                }
                Err(e) => println!("   ❌ Failed to load: {}", e),
            }

            // Clean up
            let _ = std::fs::remove_file(config_path);
        }
        Err(e) => println!("   ❌ Failed to save: {}", e),
    }

    // Territory marker types
    println!("\n🎯 Territory Marker Types:");
    let marker_types = [
        TerritoryMarkerType::Square,
        TerritoryMarkerType::Circle,
        TerritoryMarkerType::Cross,
        TerritoryMarkerType::Fill,
        TerritoryMarkerType::Overlay,
    ];

    for marker in &marker_types {
        println!(
            "   {:?} - {:?}",
            marker,
            match marker {
                TerritoryMarkerType::Square => "Small square markers",
                TerritoryMarkerType::Circle => "Small circle markers",
                TerritoryMarkerType::Cross => "Cross markers",
                TerritoryMarkerType::Fill => "Fill entire intersection",
                TerritoryMarkerType::Overlay => "Transparent overlay",
            }
        );
    }

    println!("\n✨ UI customization examples complete!");
    println!("   Run 'cargo run --bin offline_game' to see the UI in action");
}
