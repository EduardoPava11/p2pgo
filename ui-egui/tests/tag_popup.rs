//! Test for tag palette popup functionality
//! SPDX-License-Identifier: MIT OR Apache-2.0

use p2pgo_core::Coord;
use p2pgo_ui_egui::board_widget::BoardWidget;

#[test]
fn shift_click_opens_palette() {
    // Create a standalone BoardWidget for testing
    let mut board_widget = BoardWidget::new(9);
    
    // Set initial state
    board_widget.set_tag_palette(None);
    
    // Test the shift-click handler
    board_widget.handle_shift_click(Coord::new(4, 4));
    
    // Check that the palette is shown
    assert!(board_widget.get_tag_palette().is_some());
}
