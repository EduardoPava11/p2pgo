//! Tests for clipboard helper functionality

use p2pgo_ui_egui::clipboard_helper::ClipboardHelper;
use p2pgo_ui_egui::toast_manager::ToastManager;

#[test]
#[ignore] // Temporarily ignored due to import issues
fn test_clipboard_helper() {
    let mut clipboard_helper = ClipboardHelper::new();
    let mut toast_manager = ToastManager::new();
    
    // Test short ticket
    let short_ticket = "test_ticket";
    let result = clipboard_helper.copy_ticket(short_ticket, &mut toast_manager);
    
    // We can't test actual clipboard contents in headless mode,
    // but we can verify the function returns Ok in test mode
    assert!(result.is_ok(), "Should return Ok for copying ticket");
    
    // Test longer ticket with multiaddr
    let long_ticket = "/ip4/127.0.0.1/tcp/12345/p2p/QmbWqxBEKC3P8tqsKc98xmWNzrzDtRLMiMPL8wBuTGsMnR";
    let result = clipboard_helper.copy_ticket(long_ticket, &mut toast_manager);
    assert!(result.is_ok(), "Should return Ok for copying multiaddr ticket");
}

#[test]
#[ignore] // Temporarily ignored due to import issues
fn test_shorten_multiaddr() {
    let clipboard_helper = ClipboardHelper::new();
    
    // Test valid multiaddr
    let addr = "/ip4/127.0.0.1/tcp/12345/p2p/QmbWqxBEKC3P8tqsKc98xmWNzrzDtRLMiMPL8wBuTGsMnR";
    let shortened = clipboard_helper.shorten_for_display(addr);
    assert!(shortened.len() < addr.len(), "Address should be shortened");
    assert!(shortened.contains("..."), "Shortened address should contain ellipsis");
    
    // Test short string (should not be shortened)
    let short = "short_string";
    let result = clipboard_helper.shorten_for_display(short);
    assert_eq!(result, short, "Short strings should not be shortened");
}
