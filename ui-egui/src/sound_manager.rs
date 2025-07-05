//! Sound effects manager for stone placement and game events

use anyhow::Result;
use std::collections::HashMap;

/// Sound effect types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEffect {
    /// Stone placement - crisp "pop" sound
    StonePlacement,
    /// Illegal move attempt - soft thud
    IllegalMove,
    /// Stone capture - multiple quick pops
    StoneCapture,
    /// Game start chime
    GameStart,
    /// Timer warning
    TimerWarning,
    /// Move confirmation from network
    MoveConfirmed,
}

/// Sound manager for playing game audio
pub struct SoundManager {
    /// Whether sounds are enabled
    enabled: bool,
    /// Master volume (0.0 to 1.0)
    volume: f32,
    /// Per-sound volume adjustments
    sound_volumes: HashMap<SoundEffect, f32>,
    /// Platform-specific audio backend
    backend: AudioBackend,
}

/// Platform-specific audio backend
enum AudioBackend {
    /// Web Audio API for web builds
    #[cfg(target_arch = "wasm32")]
    WebAudio,
    /// Native audio for desktop
    #[cfg(not(target_arch = "wasm32"))]
    Native,
}

impl SoundManager {
    pub fn new() -> Self {
        let mut sound_volumes = HashMap::new();

        // Default volume levels for different sounds
        sound_volumes.insert(SoundEffect::StonePlacement, 0.7);
        sound_volumes.insert(SoundEffect::IllegalMove, 0.5);
        sound_volumes.insert(SoundEffect::StoneCapture, 0.6);
        sound_volumes.insert(SoundEffect::GameStart, 0.8);
        sound_volumes.insert(SoundEffect::TimerWarning, 0.9);
        sound_volumes.insert(SoundEffect::MoveConfirmed, 0.4);

        Self {
            enabled: true,
            volume: 0.8,
            sound_volumes,
            #[cfg(target_arch = "wasm32")]
            backend: AudioBackend::WebAudio,
            #[cfg(not(target_arch = "wasm32"))]
            backend: AudioBackend::Native,
        }
    }

    /// Play a sound effect
    pub fn play(&self, effect: SoundEffect) {
        if !self.enabled {
            return;
        }

        let effect_volume = self.sound_volumes.get(&effect).copied().unwrap_or(1.0);
        let final_volume = self.volume * effect_volume;

        match &self.backend {
            #[cfg(target_arch = "wasm32")]
            AudioBackend::WebAudio => {
                self.play_web_audio(effect, final_volume);
            }
            #[cfg(not(target_arch = "wasm32"))]
            AudioBackend::Native => {
                self.play_native_audio(effect, final_volume);
            }
        }
    }

    /// Play stone placement with slight pitch variation for variety
    pub fn play_stone_placement(&self, is_black: bool) {
        if !self.enabled {
            return;
        }

        // Slightly different pitch for black vs white stones
        let pitch_shift = if is_black { 0.95 } else { 1.05 };

        // For now, just play the standard sound
        // In a full implementation, we'd apply the pitch shift
        self.play(SoundEffect::StonePlacement);
    }

    /// Enable or disable all sounds
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set master volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Set volume for a specific sound effect
    pub fn set_sound_volume(&mut self, effect: SoundEffect, volume: f32) {
        self.sound_volumes.insert(effect, volume.clamp(0.0, 1.0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn play_native_audio(&self, effect: SoundEffect, volume: f32) {
        // For native platforms, we would integrate with a library like rodio
        // For now, we'll just log the sound
        tracing::debug!("Playing sound: {:?} at volume {:.2}", effect, volume);

        // Placeholder for actual audio implementation
        // In production, this would:
        // 1. Load sound files from assets
        // 2. Create audio source
        // 3. Apply volume
        // 4. Play the sound
    }

    #[cfg(target_arch = "wasm32")]
    fn play_web_audio(&self, effect: SoundEffect, volume: f32) {
        // For web builds, use the Web Audio API
        // This would be implemented with web-sys
        tracing::debug!("Playing web sound: {:?} at volume {:.2}", effect, volume);
    }
}

/// Sound effect data - represents the actual audio data
pub struct SoundData {
    /// Sample rate (e.g., 44100)
    pub sample_rate: u32,
    /// Audio samples
    pub samples: Vec<f32>,
}

impl SoundData {
    /// Generate a simple "pop" sound for stone placement
    /// This creates a short, crisp sound similar to a reversed beer can opening
    pub fn generate_stone_pop() -> Self {
        let sample_rate = 44100;
        let duration = 0.05; // 50ms
        let num_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(num_samples);

        // Create a short burst with quick attack and decay
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;

            // Envelope: very quick attack, quick decay
            let envelope = if t < 0.002 {
                // Attack (2ms)
                t / 0.002
            } else {
                // Exponential decay
                (-t * 50.0).exp()
            };

            // Mix of frequencies for a crisp "pop"
            let signal = 0.3 * (2.0 * std::f32::consts::PI * 800.0 * t).sin() +  // Mid frequency
                0.2 * (2.0 * std::f32::consts::PI * 1600.0 * t).sin() + // High frequency
                0.5 * (2.0 * std::f32::consts::PI * 400.0 * t).sin(); // Low frequency

            // Add some noise for crispness
            let noise = (i as f32 * 12345.6789).sin() * 0.1;

            samples.push((signal + noise) * envelope);
        }

        Self {
            sample_rate,
            samples,
        }
    }

    /// Generate a soft "thud" for illegal moves
    pub fn generate_illegal_thud() -> Self {
        let sample_rate = 44100;
        let duration = 0.1; // 100ms
        let num_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;

            // Softer envelope
            let envelope = (-t * 20.0).exp();

            // Lower frequencies for a "thud"
            let signal = 0.7 * (2.0 * std::f32::consts::PI * 100.0 * t).sin()
                + 0.3 * (2.0 * std::f32::consts::PI * 200.0 * t).sin();

            samples.push(signal * envelope * 0.5);
        }

        Self {
            sample_rate,
            samples,
        }
    }
}
