use eframe::egui::{self, Color32, Painter, Pos2, Rect, Ui, Vec2};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// FPS-controlled animation system for neural network visualization
pub struct NeuralAnimationSystem {
    /// Target FPS for animations
    target_fps: f32,
    /// Frame timing
    frame_timer: FrameTimer,
    /// Active animations
    animations: Vec<Box<dyn Animation>>,
    /// Animation recorder for playback
    recorder: Option<AnimationRecorder>,
}

/// Frame timer for consistent FPS
struct FrameTimer {
    last_frame: Instant,
    frame_times: VecDeque<Duration>,
    target_frame_time: Duration,
}

impl FrameTimer {
    fn new(target_fps: f32) -> Self {
        Self {
            last_frame: Instant::now(),
            frame_times: VecDeque::with_capacity(60),
            target_frame_time: Duration::from_secs_f32(1.0 / target_fps),
        }
    }
    
    fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let delta = now - self.last_frame;
        self.last_frame = now;
        
        self.frame_times.push_back(delta);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }
        
        delta.as_secs_f32()
    }
    
    fn average_fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        
        let avg_time = self.frame_times.iter()
            .map(|d| d.as_secs_f32())
            .sum::<f32>() / self.frame_times.len() as f32;
        
        1.0 / avg_time
    }
    
    fn should_render(&self) -> bool {
        self.last_frame.elapsed() >= self.target_frame_time
    }
}

/// Animation trait for different visualization effects
trait Animation: Send + Sync {
    fn update(&mut self, delta_time: f32);
    fn render(&self, painter: &Painter, rect: Rect);
    fn is_complete(&self) -> bool;
}

/// Weight flow animation showing data propagating through layers
pub struct WeightFlowAnimation {
    /// Particles flowing through the network
    particles: Vec<FlowParticle>,
    /// Network layer positions
    layer_positions: Vec<f32>,
    /// Animation progress
    progress: f32,
    /// Total duration
    duration: f32,
}

struct FlowParticle {
    start_layer: usize,
    end_layer: usize,
    start_neuron: usize,
    end_neuron: usize,
    progress: f32,
    color: Color32,
    size: f32,
}

impl WeightFlowAnimation {
    pub fn new(num_layers: usize, duration: f32) -> Self {
        let layer_positions: Vec<f32> = (0..num_layers)
            .map(|i| i as f32 / (num_layers - 1) as f32)
            .collect();
        
        Self {
            particles: Vec::new(),
            layer_positions,
            progress: 0.0,
            duration,
        }
    }
    
    pub fn add_flow(&mut self, from_layer: usize, from_neuron: usize, 
                    to_layer: usize, to_neuron: usize, strength: f32) {
        let color = if strength > 0.0 {
            Color32::from_rgba_unmultiplied(100, 200, 255, (strength * 255.0) as u8)
        } else {
            Color32::from_rgba_unmultiplied(255, 100, 100, (strength.abs() * 255.0) as u8)
        };
        
        self.particles.push(FlowParticle {
            start_layer: from_layer,
            end_layer: to_layer,
            start_neuron: from_neuron,
            end_neuron: to_neuron,
            progress: 0.0,
            color,
            size: 2.0 + strength.abs() * 4.0,
        });
    }
}

impl Animation for WeightFlowAnimation {
    fn update(&mut self, delta_time: f32) {
        self.progress += delta_time / self.duration;
        
        // Update particles
        for particle in &mut self.particles {
            particle.progress = (particle.progress + delta_time * 2.0).min(1.0);
        }
        
        // Remove completed particles
        self.particles.retain(|p| p.progress < 1.0);
        
        // Spawn new particles periodically
        if self.progress < 0.8 && self.particles.len() < 50 {
            // Add random flows
            let from_layer = 0;
            let to_layer = 1;
            self.add_flow(from_layer, rand() % 10, to_layer, rand() % 10, rand_f32());
        }
    }
    
    fn render(&self, painter: &Painter, rect: Rect) {
        for particle in &self.particles {
            let start_x = rect.left() + self.layer_positions[particle.start_layer] * rect.width();
            let end_x = rect.left() + self.layer_positions[particle.end_layer] * rect.width();
            
            let layer_height = rect.height() / 10.0;
            let start_y = rect.top() + particle.start_neuron as f32 * layer_height;
            let end_y = rect.top() + particle.end_neuron as f32 * layer_height;
            
            let x = start_x + (end_x - start_x) * particle.progress;
            let y = start_y + (end_y - start_y) * particle.progress;
            
            painter.circle_filled(Pos2::new(x, y), particle.size, particle.color);
            
            // Trail effect
            for i in 1..5 {
                let trail_progress = (particle.progress - i as f32 * 0.05).max(0.0);
                let trail_x = start_x + (end_x - start_x) * trail_progress;
                let trail_y = start_y + (end_y - start_y) * trail_progress;
                let trail_alpha = ((5 - i) as f32 / 5.0 * 0.5) * (1.0 - particle.progress);
                
                painter.circle_filled(
                    Pos2::new(trail_x, trail_y),
                    particle.size * 0.7,
                    Color32::from_rgba_unmultiplied(
                        particle.color.r(),
                        particle.color.g(),
                        particle.color.b(),
                        (particle.color.a() as f32 * trail_alpha) as u8,
                    ),
                );
            }
        }
    }
    
    fn is_complete(&self) -> bool {
        self.progress >= 1.0 && self.particles.is_empty()
    }
}

/// Activation heatmap animation
pub struct ActivationHeatmapAnimation {
    /// Grid of activation values
    activations: Vec<Vec<f32>>,
    /// Target activations (for smooth transitions)
    target_activations: Vec<Vec<f32>>,
    /// Interpolation speed
    lerp_speed: f32,
    /// Color mapping
    color_map: ColorMap,
}

struct ColorMap {
    cold: Color32,
    warm: Color32,
    hot: Color32,
}

impl ActivationHeatmapAnimation {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            activations: vec![vec![0.0; width]; height],
            target_activations: vec![vec![0.0; width]; height],
            lerp_speed: 5.0,
            color_map: ColorMap {
                cold: Color32::from_rgb(0, 0, 100),
                warm: Color32::from_rgb(255, 200, 0),
                hot: Color32::from_rgb(255, 0, 0),
            },
        }
    }
    
    pub fn set_activations(&mut self, new_activations: Vec<Vec<f32>>) {
        self.target_activations = new_activations;
    }
    
    fn value_to_color(&self, value: f32) -> Color32 {
        let v = value.clamp(0.0, 1.0);
        
        if v < 0.5 {
            // Cold to warm
            let t = v * 2.0;
            Color32::from_rgb(
                (self.cold.r() as f32 * (1.0 - t) + self.warm.r() as f32 * t) as u8,
                (self.cold.g() as f32 * (1.0 - t) + self.warm.g() as f32 * t) as u8,
                (self.cold.b() as f32 * (1.0 - t) + self.warm.b() as f32 * t) as u8,
            )
        } else {
            // Warm to hot
            let t = (v - 0.5) * 2.0;
            Color32::from_rgb(
                (self.warm.r() as f32 * (1.0 - t) + self.hot.r() as f32 * t) as u8,
                (self.warm.g() as f32 * (1.0 - t) + self.hot.g() as f32 * t) as u8,
                (self.warm.b() as f32 * (1.0 - t) + self.hot.b() as f32 * t) as u8,
            )
        }
    }
}

impl Animation for ActivationHeatmapAnimation {
    fn update(&mut self, delta_time: f32) {
        // Smooth interpolation to target
        for y in 0..self.activations.len() {
            for x in 0..self.activations[0].len() {
                let current = self.activations[y][x];
                let target = self.target_activations[y][x];
                let diff = target - current;
                self.activations[y][x] = current + diff * (delta_time * self.lerp_speed).min(1.0);
            }
        }
    }
    
    fn render(&self, painter: &Painter, rect: Rect) {
        let cell_width = rect.width() / self.activations[0].len() as f32;
        let cell_height = rect.height() / self.activations.len() as f32;
        
        for (y, row) in self.activations.iter().enumerate() {
            for (x, &value) in row.iter().enumerate() {
                let cell_rect = Rect::from_min_size(
                    Pos2::new(
                        rect.left() + x as f32 * cell_width,
                        rect.top() + y as f32 * cell_height,
                    ),
                    Vec2::new(cell_width, cell_height),
                );
                
                let color = self.value_to_color(value);
                painter.rect_filled(cell_rect, 0.0, color);
            }
        }
    }
    
    fn is_complete(&self) -> bool {
        false // Continuous animation
    }
}

/// Animation recorder for creating training visualizations
struct AnimationRecorder {
    frames: Vec<AnimationFrame>,
    recording: bool,
    max_frames: usize,
}

struct AnimationFrame {
    timestamp: f32,
    data: FrameData,
}

enum FrameData {
    Activations(Vec<Vec<f32>>),
    Weights(Vec<(usize, usize, f32)>),
    Loss(f32, f32), // policy_loss, value_loss
}

impl NeuralAnimationSystem {
    pub fn new(target_fps: f32) -> Self {
        Self {
            target_fps,
            frame_timer: FrameTimer::new(target_fps),
            animations: Vec::new(),
            recorder: None,
        }
    }
    
    /// Start recording animation frames
    pub fn start_recording(&mut self, max_frames: usize) {
        self.recorder = Some(AnimationRecorder {
            frames: Vec::new(),
            recording: true,
            max_frames,
        });
    }
    
    /// Stop recording and return frames
    pub fn stop_recording(&mut self) -> Option<Vec<AnimationFrame>> {
        self.recorder.take().map(|r| r.frames)
    }
    
    /// Add a weight flow animation
    pub fn add_weight_flow(&mut self, num_layers: usize, duration: f32) -> usize {
        let anim = Box::new(WeightFlowAnimation::new(num_layers, duration));
        self.animations.push(anim);
        self.animations.len() - 1
    }
    
    /// Add an activation heatmap
    pub fn add_activation_heatmap(&mut self, width: usize, height: usize) -> usize {
        let anim = Box::new(ActivationHeatmapAnimation::new(width, height));
        self.animations.push(anim);
        self.animations.len() - 1
    }
    
    /// Update all animations
    pub fn update(&mut self) {
        if !self.frame_timer.should_render() {
            return;
        }
        
        let delta_time = self.frame_timer.tick();
        
        // Update all animations
        for anim in &mut self.animations {
            anim.update(delta_time);
        }
        
        // Remove completed animations
        self.animations.retain(|anim| !anim.is_complete());
        
        // Record frame if recording
        if let Some(recorder) = &mut self.recorder {
            if recorder.recording && recorder.frames.len() < recorder.max_frames {
                // Record current state
                // This would capture the current neural network state
            }
        }
    }
    
    /// Render all animations
    pub fn render(&self, ui: &mut Ui) {
        let available_rect = ui.available_rect();
        let painter = ui.painter();
        
        // Render each animation
        for (i, anim) in self.animations.iter().enumerate() {
            let anim_rect = Rect::from_min_size(
                available_rect.min + Vec2::new(0.0, i as f32 * 200.0),
                Vec2::new(available_rect.width(), 180.0),
            );
            
            anim.render(painter, anim_rect);
        }
        
        // Show FPS
        ui.label(format!("FPS: {:.1}", self.frame_timer.average_fps()));
    }
    
    /// Get animation by index for updates
    pub fn get_animation_mut(&mut self, index: usize) -> Option<&mut (dyn Animation + 'static)> {
        self.animations.get_mut(index).map(|a| a.as_mut())
    }
}

// Utility functions
fn rand() -> usize {
    static mut SEED: u32 = 1234;
    unsafe {
        SEED = SEED.wrapping_mul(1664525).wrapping_add(1013904223);
        (SEED % 100) as usize
    }
}

fn rand_f32() -> f32 {
    (rand() as f32 / 100.0) * 2.0 - 1.0
}

/// Integration with training visualization
impl NeuralAnimationSystem {
    /// Update from training metrics
    pub fn update_from_training(&mut self, policy_loss: f32, value_loss: f32, epoch: u32) {
        // Update weight flow based on loss changes
        if let Some(_flow) = self.animations.get_mut(0) {
            // Add flows proportional to loss
            // Lower loss = more positive flows
        }
        
        // Record training frame
        if let Some(recorder) = &mut self.recorder {
            if recorder.recording {
                recorder.frames.push(AnimationFrame {
                    timestamp: epoch as f32,
                    data: FrameData::Loss(policy_loss, value_loss),
                });
            }
        }
    }
    
    /// Create animation from recorded frames
    pub fn create_playback_animation(&self, frames: &[AnimationFrame]) -> PlaybackAnimation {
        PlaybackAnimation {
            frames: frames.to_vec(),
            current_frame: 0,
            playback_speed: 1.0,
            looping: true,
        }
    }
}

/// Playback animation for recorded training sessions
pub struct PlaybackAnimation {
    frames: Vec<AnimationFrame>,
    current_frame: usize,
    playback_speed: f32,
    looping: bool,
}

impl Animation for PlaybackAnimation {
    fn update(&mut self, delta_time: f32) {
        self.current_frame = ((self.current_frame as f32 + delta_time * self.playback_speed * 30.0) as usize) 
            % self.frames.len();
    }
    
    fn render(&self, painter: &Painter, rect: Rect) {
        if let Some(frame) = self.frames.get(self.current_frame) {
            match &frame.data {
                FrameData::Loss(policy, value) => {
                    // Render loss visualization
                    let text = format!("Epoch {:.0}: P={:.4} V={:.4}", 
                        frame.timestamp, policy, value);
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(14.0),
                        Color32::WHITE,
                    );
                }
                _ => {}
            }
        }
    }
    
    fn is_complete(&self) -> bool {
        !self.looping && self.current_frame >= self.frames.len() - 1
    }
}