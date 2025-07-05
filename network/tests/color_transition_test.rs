//! Tests for colorblind-safe color transitions in relay status

use p2pgo_core::color_constants::relay_status;
use p2pgo_network::relay_monitor::{relay_health_color, RelayHealthStatus};

#[test]
fn test_relay_status_colors_are_colorblind_friendly() {
    // Get colors for different states
    let healthy_color = relay_health_color(&RelayHealthStatus::Healthy);
    let degraded_color = relay_health_color(&RelayHealthStatus::Degraded);
    let unreachable_color = relay_health_color(&RelayHealthStatus::Unreachable);
    let restarting_color = relay_health_color(&RelayHealthStatus::Restarting);
    let failed_color = relay_health_color(&RelayHealthStatus::Failed);

    // Verify they match our Okabe-Ito palette constants
    assert_eq!(healthy_color.as_rgb(), relay_status::HEALTHY);
    assert_eq!(degraded_color.as_rgb(), relay_status::DEGRADED);
    assert_eq!(unreachable_color.as_rgb(), relay_status::OFFLINE);
    assert_eq!(restarting_color.as_rgb(), relay_status::RESTARTING);
    assert_eq!(failed_color.as_rgb(), relay_status::ERROR);

    // Check color contrast to ensure they're distinguishable (basic test)
    // This is a very simplified check - proper contrast would use more sophisticated methods

    // Helper to calculate contrast (basic Euclidean distance in RGB space)
    fn color_distance(c1: [f32; 3], c2: [f32; 3]) -> f32 {
        let dr = c1[0] - c2[0];
        let dg = c1[1] - c2[1];
        let db = c1[2] - c2[2];
        (dr * dr + dg * dg + db * db).sqrt()
    }

    // Each color should be sufficiently different from the others
    let min_contrast = 0.15; // Minimum acceptable contrast in RGB space

    assert!(color_distance(healthy_color.as_rgb(), degraded_color.as_rgb()) > min_contrast);
    assert!(color_distance(healthy_color.as_rgb(), unreachable_color.as_rgb()) > min_contrast);
    assert!(color_distance(healthy_color.as_rgb(), restarting_color.as_rgb()) > min_contrast);
    assert!(color_distance(healthy_color.as_rgb(), failed_color.as_rgb()) > min_contrast);

    assert!(color_distance(degraded_color.as_rgb(), unreachable_color.as_rgb()) > min_contrast);
    assert!(color_distance(degraded_color.as_rgb(), restarting_color.as_rgb()) > min_contrast);
    assert!(color_distance(degraded_color.as_rgb(), failed_color.as_rgb()) > min_contrast);

    assert!(color_distance(unreachable_color.as_rgb(), restarting_color.as_rgb()) > min_contrast);
    assert!(color_distance(unreachable_color.as_rgb(), failed_color.as_rgb()) > min_contrast);

    assert!(color_distance(restarting_color.as_rgb(), failed_color.as_rgb()) > min_contrast);
}
