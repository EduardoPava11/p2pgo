//! Stone placement animation system for satisfying visual feedback

use egui::{Color32, Pos2, Vec2, Rect};
use std::time::{Duration, Instant};
use p2pgo_core::{Color as StoneColor, Coord};

/// Animation state for a single stone
#[derive(Clone, Debug)]
pub struct StoneAnimation {
    /// Board coordinate of the stone
    pub coord: Coord,
    /// Stone color
    pub color: StoneColor,
    /// Animation start time
    pub start_time: Instant,
    /// Animation duration
    pub duration: Duration,
    /// Animation type
    pub animation_type: AnimationType,
    /// Current animation progress (0.0 to 1.0)
    pub progress: f32,
}

/// Types of stone animations
#[derive(Clone, Debug, PartialEq)]
pub enum AnimationType {
    /// Stone being placed with drop effect
    Placement {
        start_height: f32,
        bounce_count: u8,
    },
    /// Stone captured and fading out
    Capture {
        target_scale: f32,
    },
    /// Hover preview
    HoverPreview,
    /// Last move indicator pulse
    LastMoveIndicator,
    /// Network pending state
    PendingConfirmation,
    /// Move rejected
    Rejected,
}

impl StoneAnimation {
    /// Create a new placement animation
    pub fn new_placement(coord: Coord, color: StoneColor) -> Self {
        Self {
            coord,
            color,
            start_time: Instant::now(),
            duration: Duration::from_millis(200), // Faster for smoother feel
            animation_type: AnimationType::Placement {
                start_height: 30.0, // Lower height for less motion
                bounce_count: 0, // No bounce to reduce complexity
            },
            progress: 0.0,
        }
    }
    
    /// Create a hover preview animation
    pub fn new_hover(coord: Coord, color: StoneColor) -> Self {
        Self {
            coord,
            color,
            start_time: Instant::now(),
            duration: Duration::from_millis(100),
            animation_type: AnimationType::HoverPreview,
            progress: 0.0,
        }
    }
    
    /// Create a capture animation
    pub fn new_capture(coord: Coord, color: StoneColor) -> Self {
        Self {
            coord,
            color,
            start_time: Instant::now(),
            duration: Duration::from_millis(400),
            animation_type: AnimationType::Capture {
                target_scale: 0.0,
            },
            progress: 0.0,
        }
    }
    
    /// Create a pending confirmation animation
    pub fn new_pending(coord: Coord, color: StoneColor) -> Self {
        Self {
            coord,
            color,
            start_time: Instant::now(),
            duration: Duration::from_millis(2000), // Longer for network wait
            animation_type: AnimationType::PendingConfirmation,
            progress: 0.0,
        }
    }
    
    /// Update animation progress
    pub fn update(&mut self) -> bool {
        let elapsed = self.start_time.elapsed();
        self.progress = (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0);
        
        // Return true if animation is complete
        self.progress >= 1.0
    }
    
    /// Get the current animation transform
    pub fn get_transform(&self, base_pos: Pos2, stone_radius: f32) -> AnimationTransform {
        match &self.animation_type {
            AnimationType::Placement { start_height, bounce_count } => {
                // Ease out quad for main drop
                let mut y_offset = -*start_height * (1.0 - ease_out_quad(self.progress));
                
                // Add bounce effect
                if *bounce_count > 0 && self.progress > 0.7 {
                    let bounce_progress = (self.progress - 0.7) / 0.3;
                    let bounce_height = 5.0 * (1.0 - bounce_progress);
                    y_offset -= bounce_height * ease_out_quad(bounce_progress);
                }
                
                AnimationTransform {
                    position: base_pos + Vec2::new(0.0, y_offset),
                    scale: 1.0,
                    opacity: ease_in_quad(self.progress.min(0.3) / 0.3),
                    rotation: 0.0,
                }
            }
            
            AnimationType::Capture { target_scale } => {
                // Shrink and fade out
                let scale = 1.0 - (1.0 - target_scale) * ease_in_quad(self.progress);
                let opacity = 1.0 - ease_in_quad(self.progress);
                
                AnimationTransform {
                    position: base_pos,
                    scale,
                    opacity,
                    rotation: self.progress * std::f32::consts::PI * 0.5, // Slight rotation
                }
            }
            
            AnimationType::HoverPreview => {
                // Fade in quickly
                AnimationTransform {
                    position: base_pos,
                    scale: 1.0,
                    opacity: 0.5 * ease_out_quad(self.progress),
                    rotation: 0.0,
                }
            }
            
            AnimationType::LastMoveIndicator => {
                // Pulsing effect
                let pulse = (self.progress * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                AnimationTransform {
                    position: base_pos,
                    scale: 1.0 + pulse * 0.1,
                    opacity: 1.0,
                    rotation: 0.0,
                }
            }
            
            AnimationType::PendingConfirmation => {
                // Gentle pulsing opacity
                let pulse = (self.progress * std::f32::consts::TAU * 2.0).sin() * 0.25 + 0.75;
                AnimationTransform {
                    position: base_pos,
                    scale: 1.0,
                    opacity: pulse,
                    rotation: 0.0,
                }
            }
            
            AnimationType::Rejected => {
                // Shake and fade out
                let shake_x = if self.progress < 0.5 {
                    (self.progress * 20.0).sin() * 5.0 * (1.0 - self.progress * 2.0)
                } else {
                    0.0
                };
                
                AnimationTransform {
                    position: base_pos + Vec2::new(shake_x, 0.0),
                    scale: 1.0,
                    opacity: 1.0 - ease_in_quad((self.progress - 0.5).max(0.0) * 2.0),
                    rotation: 0.0,
                }
            }
        }
    }
    
    /// Get ripple effect for placement animations
    pub fn get_ripple(&self) -> Option<RippleEffect> {
        match &self.animation_type {
            AnimationType::Placement { .. } => {
                if self.progress < 0.6 {
                    let ripple_progress = self.progress / 0.6;
                    Some(RippleEffect {
                        radius_factor: 1.0 + ripple_progress * 0.5,
                        opacity: (1.0 - ripple_progress) * 0.3,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Transform values for rendering
#[derive(Clone, Debug)]
pub struct AnimationTransform {
    pub position: Pos2,
    pub scale: f32,
    pub opacity: f32,
    pub rotation: f32,
}

/// Ripple effect parameters
#[derive(Clone, Debug)]
pub struct RippleEffect {
    pub radius_factor: f32,
    pub opacity: f32,
}

/// Easing functions for smooth animations
fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_in_quad(t: f32) -> f32 {
    t * t
}

fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - 2.0 * (1.0 - t) * (1.0 - t)
    }
}

/// Animation manager for the board
pub struct AnimationManager {
    /// Active animations
    animations: Vec<StoneAnimation>,
    /// Maximum concurrent animations
    max_animations: usize,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
            max_animations: 10,
        }
    }
    
    /// Add a new animation
    pub fn add_animation(&mut self, animation: StoneAnimation) {
        // Remove oldest animations if at limit
        if self.animations.len() >= self.max_animations {
            self.animations.remove(0);
        }
        
        // Remove any existing animation for the same coordinate
        self.animations.retain(|a| a.coord != animation.coord || 
                               matches!(a.animation_type, AnimationType::LastMoveIndicator));
        
        self.animations.push(animation);
    }
    
    /// Update all animations, removing completed ones
    pub fn update(&mut self) -> bool {
        // Don't do frame rate limiting here - let egui handle it
        // This was causing glitchy animations
        
        // Update animations
        self.animations.retain_mut(|anim| !anim.update());
        
        // Clean up if we have too many animations
        if self.animations.len() > self.max_animations {
            // Keep only the most recent animations
            let start = self.animations.len().saturating_sub(self.max_animations);
            self.animations.drain(0..start);
        }
        
        self.has_animations()
    }
    
    /// Get animation for a specific coordinate
    pub fn get_animation(&self, coord: &Coord) -> Option<&StoneAnimation> {
        self.animations.iter()
            .find(|a| &a.coord == coord)
    }
    
    /// Get all active animations
    pub fn get_animations(&self) -> &[StoneAnimation] {
        &self.animations
    }
    
    /// Clear all animations
    pub fn clear(&mut self) {
        self.animations.clear();
    }
    
    /// Check if any animations are active
    pub fn has_animations(&self) -> bool {
        !self.animations.is_empty()
    }
}