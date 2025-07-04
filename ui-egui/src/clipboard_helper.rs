// SPDX-License-Identifier: MIT OR Apache-2.0

//! Clipboard helper utilities for P2P Go

use eframe::egui;
use anyhow::{Result, anyhow};

/// Clipboard helper for cross-platform clipboard access
pub struct ClipboardHelper {
    #[cfg(feature = "arboard")]
    clipboard: Option<arboard::Clipboard>,
}

impl Default for ClipboardHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardHelper {
    /// Create a new clipboard helper
    pub fn new() -> Self {
        #[cfg(feature = "arboard")]
        let clipboard = match arboard::Clipboard::new() {
            Ok(clipboard) => Some(clipboard),
            Err(e) => {
                tracing::warn!("Failed to initialize clipboard: {}", e);
                None
            }
        };

        Self {
            #[cfg(feature = "arboard")]
            clipboard,
        }
    }
    
    /// Set the clipboard text
    pub fn set_text(&mut self, text: &str, ctx: Option<&egui::Context>) -> Result<()> {
        // First try using the platform clipboard with arboard
        #[cfg(feature = "arboard")]
        if let Some(clipboard) = &mut self.clipboard {
            return clipboard.set_text(text)
                .map_err(|e| anyhow!("Failed to set clipboard text: {}", e));
        }
        
        // Otherwise fallback to egui clipboard (may not work as well across applications)
        if let Some(ctx) = ctx {
            ctx.output_mut(|output| {
                output.copied_text = text.to_string();
            });
            return Ok(());
        }
        
        Err(anyhow!("No clipboard is available"))
    }
    
    /// Get the clipboard text
    pub fn get_text(&mut self, ctx: Option<&egui::Context>) -> Result<String> {
        // First try using the platform clipboard with arboard
        #[cfg(feature = "arboard")]
        if let Some(clipboard) = &mut self.clipboard {
            return clipboard.get_text()
                .map_err(|e| anyhow!("Failed to get clipboard text: {}", e));
        }
        
        // Otherwise fallback to egui clipboard
        if let Some(ctx) = ctx {
            if let Some(text) = ctx.input(|input| input.events.iter().find_map(|event| {
                if let egui::Event::Paste(text) = event {
                    Some(text.clone())
                } else {
                    None
                }
            })) {
                return Ok(text);
            }
        }
        
        Err(anyhow!("No clipboard text available"))
    }
    
    /// Copy a ticket to the clipboard
    pub fn copy_ticket(&mut self, ticket: &str, toast_manager: &mut crate::toast_manager::ToastManager) -> Result<bool> {
        // Only show toast for real tickets (sufficiently complex)
        if ticket.len() < 50 {
            // Simple address, just copy without toast
            #[cfg(not(test))]
            {
                return self.set_text(ticket, None).map(|_| false);
            }
            
            #[cfg(test)]
            {
                // In test mode, just return success without actually copying
                return Ok(false);
            }
        }
        
        // Format as a pretty ticket with instructions
        let _pretty_ticket = format!(
            "=== P2P GO TICKET ===\n\
            {}\n\
            ======================\n\
            Paste this ticket in the P2P Go app to join the game.",
            ticket
        );
        
        // Add toast notification
        toast_manager.add_toast(
            "Connection ticket copied to clipboard!",
            crate::toast_manager::ToastType::Success,
        );
        
        #[cfg(not(test))]
        {
            self.set_text(ticket, None).map(|_| true)
        }
        
        #[cfg(test)]
        {
            // In test mode, just return success without actually copying
            Ok(true)
        }
    }
    
    /// Copy a ticket to the clipboard and show toast
    pub fn copy_ticket_with_toast(&mut self, ticket: &str, toast_manager: &mut crate::toast_manager::ToastManager) -> Result<()> {
        // Check if ticket is actually empty or a stub
        if ticket.is_empty() || ticket.len() < 20 {
            toast_manager.add_toast(
                "No valid ticket available",
                crate::toast_manager::ToastType::Error,
            );
            return Ok(());
        }
        
        // Copy full ticket (includes multiaddr + public key in v0.35)
        match self.copy_ticket(ticket, toast_manager) {
            Ok(_) => {
                // Toast already shown in copy_ticket
                Ok(())
            },
            Err(e) => {
                toast_manager.add_toast(
                    "Failed to copy ticket",
                    crate::toast_manager::ToastType::Error,
                );
                Err(e)
            }
        }
    }
    
    /// Shorten a multiaddr for display
    pub fn shorten_for_display(&self, addr: &str) -> String {
        if addr.len() < 30 {
            return addr.to_string();
        }
        
        // Check if it's a multiaddr
        if addr.starts_with("/ip") {
            // Find the peer ID part at the end
            if let Some(p2p_index) = addr.rfind("/p2p/") {
                let peer_id = &addr[p2p_index + 5..];
                
                // Take first 8 chars of peer ID
                let shortened_peer_id = if peer_id.len() > 8 {
                    format!("{}...", &peer_id[..8])
                } else {
                    peer_id.to_string()
                };
                
                // Include address type
                let mut address_type = String::new();
                if addr.contains("/tcp/") {
                    address_type.push_str("TCP");
                } else if addr.contains("/udp/") {
                    address_type.push_str("UDP");
                }
                
                // Find port if present
                let port = if addr.contains("/tcp/") {
                    if let Some(tcp_index) = addr.find("/tcp/") {
                        let port_end = addr[tcp_index + 5..].find('/').unwrap_or(addr[tcp_index + 5..].len());
                        Some(&addr[tcp_index + 5..tcp_index + 5 + port_end])
                    } else {
                        None
                    }
                } else if addr.contains("/udp/") {
                    if let Some(udp_index) = addr.find("/udp/") {
                        let port_end = addr[udp_index + 5..].find('/').unwrap_or(addr[udp_index + 5..].len());
                        Some(&addr[udp_index + 5..udp_index + 5 + port_end])
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Format shortened display
                if let Some(port_str) = port {
                    return format!("{} ({}:{}) [{}]", address_type, "...", port_str, shortened_peer_id);
                } else {
                    return format!("{} [{}]", address_type, shortened_peer_id);
                }
            }
        }
        
        // Default shortening for non-multiaddr
        format!("{}...{}", &addr[..10], &addr[addr.len() - 8..])
    }
}



/// Try to extract a ticket from arbitrary text
pub fn parse_ticket_from_text(text: &str) -> Result<String> {
    // First check for multiaddr format (starts with "/")
    if text.trim().starts_with('/') {
        let multiaddr_line = text.trim().lines().next().unwrap_or(text.trim());
        if multiaddr_line.len() > 10 {
            return Ok(multiaddr_line.to_string());
        }
    }

    // Try to find a line that looks like a base64 encoded ticket
    for line in text.lines() {
        let trimmed = line.trim();
        
        // Tickets are base64 encoded and usually around 100-300 chars
        if trimmed.len() > 50 && 
           trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=') {
            return Ok(trimmed.to_string());
        }
    }
    
    // Also look for text between ticket markers
    if let (Some(start), Some(end)) = (
        text.find("=== P2P GO TICKET ==="), 
        text.find("======================")
    ) {
        // Extract the text between the markers
        let marker_text = &text[start + 21..end];
        let trimmed = marker_text.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    
    Err(anyhow!("No valid ticket found in the text"))
}
