// SPDX-License-Identifier: MIT OR Apache-2.0

//! Toast notifications manager for P2P Go UI

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use eframe::egui::{self, Color32, Vec2, Align2, RichText};
use p2pgo_core::color_constants::okabe_ito;

/// Toast notification type
#[derive(Debug, Clone, PartialEq)]
pub enum ToastType {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastType {
    /// Get the color for this toast type using colorblind-safe Okabe-Ito palette
    pub fn color(&self) -> Color32 {
        match self {
            ToastType::Info => Color32::from_rgb_additive(
                (okabe_ito::BLUE[0] * 255.0) as u8,
                (okabe_ito::BLUE[1] * 255.0) as u8, 
                (okabe_ito::BLUE[2] * 255.0) as u8),
            ToastType::Success => Color32::from_rgb_additive(
                (okabe_ito::GREEN[0] * 255.0) as u8,
                (okabe_ito::GREEN[1] * 255.0) as u8, 
                (okabe_ito::GREEN[2] * 255.0) as u8),
            ToastType::Warning => Color32::from_rgb_additive(
                (okabe_ito::ORANGE[0] * 255.0) as u8,
                (okabe_ito::ORANGE[1] * 255.0) as u8, 
                (okabe_ito::ORANGE[2] * 255.0) as u8),
            ToastType::Error => Color32::from_rgb_additive(
                (okabe_ito::VERMILLION[0] * 255.0) as u8,
                (okabe_ito::VERMILLION[1] * 255.0) as u8, 
                (okabe_ito::VERMILLION[2] * 255.0) as u8),
        }
    }
    
    /// Get the icon for this toast type
    pub fn icon(&self) -> &str {
        match self {
            ToastType::Info => "ℹ️",
            ToastType::Success => "✅",
            ToastType::Warning => "⚠️", 
            ToastType::Error => "❌",
        }
    }
}

/// A single toast notification
#[derive(Debug, Clone)]
pub struct Toast {
    message: String,
    toast_type: ToastType,
    created_at: Instant,
    duration: Duration,
}

impl Toast {
    /// Create a new toast notification
    pub fn new(message: impl Into<String>, toast_type: ToastType) -> Self {
        Self {
            message: message.into(),
            toast_type,
            created_at: Instant::now(),
            duration: Duration::from_secs(4),  // Default 4 seconds
        }
    }
    
    /// Set a custom duration for this toast
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
    
    /// Check if the toast has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.duration
    }
    
    /// Calculate the fade-out factor (0.0 = fully visible, 1.0 = fully transparent)
    /// In the last second of display, the toast will fade out
    pub fn fade_factor(&self) -> f32 {
        let elapsed = self.created_at.elapsed();
        if elapsed + Duration::from_secs(1) > self.duration {
            let fade_duration = Duration::from_secs(1).as_secs_f32();
            let time_left = (self.duration - elapsed).as_secs_f32();
            1.0 - (time_left / fade_duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Toast notification manager
pub struct ToastManager {
    toasts: VecDeque<Toast>,
    max_toasts: usize,
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastManager {
    /// Create a new toast manager
    pub fn new() -> Self {
        Self {
            toasts: VecDeque::new(),
            max_toasts: 3,  // Show at most 3 toasts at once
        }
    }
    
    /// Add a new info toast
    pub fn info(&mut self, message: impl Into<String>) {
        self.add(Toast::new(message, ToastType::Info));
    }
    
    /// Add a new success toast
    pub fn success(&mut self, message: impl Into<String>) {
        self.add(Toast::new(message, ToastType::Success));
    }
    
    /// Add a new warning toast
    pub fn warning(&mut self, message: impl Into<String>) {
        self.add(Toast::new(message, ToastType::Warning));
    }
    
    /// Add a new error toast
    pub fn error(&mut self, message: impl Into<String>) {
        self.add(Toast::new(message, ToastType::Error));
    }
    
    /// Add a custom toast
    pub fn add(&mut self, toast: Toast) {
        // Enforce maximum number of toasts
        while self.toasts.len() >= self.max_toasts {
            self.toasts.pop_front();
        }
        
        self.toasts.push_back(toast);
    }
    
    /// Convenience method to add a new toast with the given message and level
    pub fn add_toast(&mut self, message: impl Into<String>, level: ToastType) {
        self.add(Toast::new(message, level));
    }

    /// Update the toast manager state and remove expired toasts, passing context for animations
    pub fn update(&mut self, ctx: &egui::Context) {
        self.toasts.retain(|toast| !toast.is_expired());
        ctx.request_repaint(); // Request repaint to ensure animations run smoothly
    }
    
    /// Original update method without context, now delegating to the context version with a dummy context
    #[allow(dead_code)]
    pub fn update_internal(&mut self) {
        self.toasts.retain(|toast| !toast.is_expired());
    }
    
    /// Draw all active toasts
    pub fn show(&mut self, ctx: &egui::Context) {
        if self.toasts.is_empty() {
            return;
        }
        
        // Update and remove expired toasts
        self.update(ctx);
        
        let mut remove_indices = Vec::new();
        
        // Show the toasts in a top-right panel
        for (idx, toast) in self.toasts.iter().enumerate() {
            let fade_factor = toast.fade_factor();
            let opacity = 1.0 - fade_factor;
            
            if fade_factor >= 1.0 {
                remove_indices.push(idx);
                continue;
            }
            
            // Calculate position (stacked from top)
            let pos_y = 10.0 + (idx as f32 * 60.0);
            
            // Use a more accessible color scheme
            let bg_color = Color32::from_black_alpha((200.0 * opacity) as u8);
            
            egui::Window::new(format!("##toast_{}", idx))
                .fixed_pos(egui::pos2(ctx.screen_rect().right() - 20.0, pos_y))
                .anchor(Align2::RIGHT_TOP, Vec2::new(0.0, 0.0))
                .title_bar(false)
                .resizable(false)
                .movable(false)
                .frame(egui::Frame::none().fill(bg_color))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        // Icon
                        ui.label(RichText::new(toast.toast_type.icon())
                            .color(toast.toast_type.color().linear_multiply(opacity)));
                        
                        // Message
                        ui.label(RichText::new(&toast.message)
                            .color(egui::Color32::WHITE.linear_multiply(opacity))
                            .strong());
                    });
                });
        }
        
        // Remove expired toasts (in reverse order to keep indices valid)
        for idx in remove_indices.iter().rev() {
            if *idx < self.toasts.len() {
                self.toasts.remove(*idx);
            }
        }
    }
    
    /// Check if there are any active toasts
    pub fn has_toasts(&self) -> bool {
        !self.toasts.is_empty()
    }
    
    /// Clear all toasts immediately
    pub fn clear(&mut self) {
        self.toasts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_toast_creation() {
        let toast = Toast::new("Test toast", ToastType::Info);
        assert_eq!(toast.message, "Test toast");
        assert_eq!(toast.toast_type, ToastType::Info);
        assert!(!toast.is_expired());
    }
    
    #[test]
    fn test_toast_expiry() {
        let toast = Toast::new("Short toast", ToastType::Warning)
            .with_duration(Duration::from_millis(5));
        
        // Sleep to ensure expiry
        std::thread::sleep(Duration::from_millis(10));
        
        assert!(toast.is_expired());
        assert_eq!(toast.fade_factor(), 1.0);
    }
    
    #[test]
    fn test_toast_manager() {
        let mut manager = ToastManager::new();
        assert!(!manager.has_toasts());
        
        manager.info("Info toast");
        manager.success("Success toast");
        assert!(manager.has_toasts());
        assert_eq!(manager.toasts.len(), 2);
        
        // Test max toasts
        manager.warning("Warning 1");
        manager.error("Error 1");
        manager.info("Info 2");
        assert_eq!(manager.toasts.len(), 3); // Should be capped
        
        // Test clear
        manager.clear();
        assert!(!manager.has_toasts());
    }
}
