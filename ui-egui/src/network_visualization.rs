use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// RNA packet visualization
#[derive(Clone)]
pub struct RNAPacket {
    pub id: u64,
    pub rna_type: RNAVisualizationType,
    pub source: Pos2,
    pub destination: Pos2,
    pub progress: f32,
    pub size: f32,
    pub color: Color32,
    pub created_at: Instant,
    pub data_size_kb: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RNAVisualizationType {
    GameData,      // mRNA - full games
    PatternData,   // tRNA - patterns
    ModelWeights,  // Neural net updates
    Discovery,     // Network discovery
}

/// Network activity visualization
pub struct NetworkVisualization {
    /// Active RNA packets
    packets: HashMap<u64, RNAPacket>,
    /// Packet ID counter
    next_packet_id: u64,
    /// Network statistics
    stats: NetworkStats,
    /// Animation phase
    animation_phase: f32,
    /// Relay positions
    relay_positions: HashMap<String, RelayNode>,
    /// Connection lines
    connections: Vec<Connection>,
    /// History for charts
    bandwidth_history: VecDeque<(Instant, f32, f32)>, // (time, sent, received)
}

#[derive(Clone)]
pub struct RelayNode {
    pub id: String,
    pub position: Pos2,
    pub is_local: bool,
    pub health: f32,
    pub last_activity: Instant,
    pub data_sent_kb: f32,
    pub data_received_kb: f32,
}

#[derive(Clone)]
pub struct Connection {
    pub from: String,
    pub to: String,
    pub strength: f32,
    pub connection_type: ConnectionType,
}

#[derive(Clone, Copy)]
pub enum ConnectionType {
    Direct,
    Relayed,
    Local,
}

#[derive(Default)]
pub struct NetworkStats {
    pub total_sent_kb: f32,
    pub total_received_kb: f32,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub active_connections: usize,
    pub current_bandwidth_kbps: f32,
}

impl NetworkVisualization {
    pub fn new() -> Self {
        Self {
            packets: HashMap::new(),
            next_packet_id: 0,
            stats: NetworkStats::default(),
            animation_phase: 0.0,
            relay_positions: HashMap::new(),
            connections: Vec::new(),
            bandwidth_history: VecDeque::with_capacity(60),
        }
    }
    
    /// Add a relay node to visualization
    pub fn add_relay(&mut self, id: String, position: Pos2, is_local: bool) {
        self.relay_positions.insert(id.clone(), RelayNode {
            id,
            position,
            is_local,
            health: 1.0,
            last_activity: Instant::now(),
            data_sent_kb: 0.0,
            data_received_kb: 0.0,
        });
    }
    
    /// Add connection between relays
    pub fn add_connection(&mut self, from: String, to: String, connection_type: ConnectionType) {
        self.connections.push(Connection {
            from,
            to,
            strength: 1.0,
            connection_type,
        });
    }
    
    /// Send RNA packet animation
    pub fn send_packet(
        &mut self,
        from: &str,
        to: &str,
        rna_type: RNAVisualizationType,
        data_size_kb: f32,
    ) {
        if let (Some(from_node), Some(to_node)) = (
            self.relay_positions.get(from),
            self.relay_positions.get(to)
        ) {
            let packet = RNAPacket {
                id: self.next_packet_id,
                rna_type,
                source: from_node.position,
                destination: to_node.position,
                progress: 0.0,
                size: (data_size_kb.log2() + 1.0).max(5.0).min(20.0),
                color: match rna_type {
                    RNAVisualizationType::GameData => Color32::from_rgb(100, 200, 255),
                    RNAVisualizationType::PatternData => Color32::from_rgb(255, 200, 100),
                    RNAVisualizationType::ModelWeights => Color32::from_rgb(200, 100, 255),
                    RNAVisualizationType::Discovery => Color32::from_rgb(100, 255, 100),
                },
                created_at: Instant::now(),
                data_size_kb,
            };
            
            self.packets.insert(self.next_packet_id, packet);
            self.next_packet_id += 1;
            
            // Update stats
            self.stats.packets_sent += 1;
            self.stats.total_sent_kb += data_size_kb;
            
            // Update node stats
            if let Some(node) = self.relay_positions.get_mut(from) {
                node.data_sent_kb += data_size_kb;
                node.last_activity = Instant::now();
            }
        }
    }
    
    /// Render the network visualization
    pub fn render(&mut self, ui: &mut Ui) {
        let available_size = ui.available_size();
        let available_rect = egui::Rect::from_min_size(ui.cursor().min, available_size);
        let response = ui.allocate_rect(available_rect, Sense::hover());
        let painter = ui.painter();
        
        // Update animation
        self.animation_phase += ui.input(|i| i.unstable_dt);
        
        // Update bandwidth history
        let now = Instant::now();
        if self.bandwidth_history.is_empty() || 
           now.duration_since(self.bandwidth_history.back().unwrap().0) > Duration::from_secs(1) {
            self.bandwidth_history.push_back((now, self.stats.total_sent_kb, self.stats.total_received_kb));
            if self.bandwidth_history.len() > 60 {
                self.bandwidth_history.pop_front();
            }
        }
        
        // Draw connections
        for connection in &self.connections {
            if let (Some(from), Some(to)) = (
                self.relay_positions.get(&connection.from),
                self.relay_positions.get(&connection.to)
            ) {
                let stroke = match connection.connection_type {
                    ConnectionType::Direct => Stroke::new(2.0, Color32::from_rgba_unmultiplied(100, 200, 100, 100)),
                    ConnectionType::Relayed => Stroke::new(1.5, Color32::from_rgba_unmultiplied(200, 200, 100, 80)),
                    ConnectionType::Local => Stroke::new(2.5, Color32::from_rgba_unmultiplied(100, 100, 200, 120)),
                };
                
                painter.line_segment([from.position, to.position], stroke);
            }
        }
        
        // Draw and update packets
        let mut completed_packets = Vec::new();
        for (id, packet) in &mut self.packets {
            packet.progress += ui.input(|i| i.unstable_dt) * 0.5; // Adjust speed
            
            if packet.progress >= 1.0 {
                completed_packets.push(*id);
                
                // Update receive stats
                if let Some(to_node) = self.relay_positions.get_mut(
                    &self.relay_positions.iter()
                        .find(|(_, n)| n.position == packet.destination)
                        .map(|(id, _)| id.clone())
                        .unwrap_or_default()
                ) {
                    to_node.data_received_kb += packet.data_size_kb;
                    self.stats.packets_received += 1;
                    self.stats.total_received_kb += packet.data_size_kb;
                }
            } else {
                // Interpolate position
                let pos = packet.source + (packet.destination - packet.source) * packet.progress;
                
                // Draw packet
                painter.circle_filled(pos, packet.size, packet.color);
                
                // Pulsing effect
                let pulse = (self.animation_phase * 5.0 + packet.id as f32).sin() * 0.3 + 1.0;
                painter.circle_stroke(
                    pos,
                    packet.size * pulse,
                    Stroke::new(1.0, packet.color.gamma_multiply(0.5)),
                );
                
                // Data size label
                if packet.data_size_kb > 10.0 {
                    painter.text(
                        pos + Vec2::new(0.0, packet.size + 5.0),
                        egui::Align2::CENTER_TOP,
                        format!("{:.0}KB", packet.data_size_kb),
                        FontId::proportional(10.0),
                        Color32::GRAY,
                    );
                }
            }
        }
        
        // Remove completed packets
        for id in completed_packets {
            self.packets.remove(&id);
        }
        
        // Draw relay nodes
        for (id, node) in &self.relay_positions {
            let age = node.last_activity.elapsed().as_secs_f32();
            let activity_alpha = (1.0 - age / 10.0).clamp(0.3, 1.0);
            
            // Node body
            let color = if node.is_local {
                Color32::from_rgb(100, 200, 255)
            } else {
                Color32::from_rgb(200, 200, 200)
            };
            
            painter.circle_filled(
                node.position,
                20.0,
                color.gamma_multiply(activity_alpha),
            );
            
            // Health indicator
            let health_color = Color32::from_rgb(
                ((1.0 - node.health) * 255.0) as u8,
                (node.health * 255.0) as u8,
                0,
            );
            painter.circle_stroke(
                node.position,
                22.0,
                Stroke::new(2.0, health_color),
            );
            
            // Node label
            painter.text(
                node.position,
                egui::Align2::CENTER_CENTER,
                &id[..8.min(id.len())],
                FontId::monospace(10.0),
                Color32::WHITE,
            );
            
            // Data stats
            painter.text(
                node.position + Vec2::new(0.0, 25.0),
                egui::Align2::CENTER_TOP,
                format!("↑{:.0} ↓{:.0}", node.data_sent_kb, node.data_received_kb),
                FontId::proportional(9.0),
                Color32::GRAY,
            );
        }
        
        // Draw stats panel
        self.render_stats_panel(ui, response.rect.right_top() + Vec2::new(-150.0, 10.0));
    }
    
    fn render_stats_panel(&self, ui: &mut Ui, pos: Pos2) {
        let painter = ui.painter();
        let panel_rect = Rect::from_min_size(pos, Vec2::new(140.0, 120.0));
        
        // Background
        painter.rect_filled(
            panel_rect,
            5.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 200),
        );
        
        // Stats text
        let stats = [
            format!("Sent: {:.1} KB", self.stats.total_sent_kb),
            format!("Recv: {:.1} KB", self.stats.total_received_kb),
            format!("Packets: {} / {}", self.stats.packets_sent, self.stats.packets_received),
            format!("Connections: {}", self.connections.len()),
        ];
        
        for (i, stat) in stats.iter().enumerate() {
            painter.text(
                pos + Vec2::new(10.0, 10.0 + i as f32 * 15.0),
                egui::Align2::LEFT_TOP,
                stat,
                FontId::proportional(12.0),
                Color32::LIGHT_GRAY,
            );
        }
        
        // Mini bandwidth chart
        if self.bandwidth_history.len() > 2 {
            let chart_rect = Rect::from_min_size(
                pos + Vec2::new(10.0, 70.0),
                Vec2::new(120.0, 40.0),
            );
            
            painter.rect_stroke(chart_rect, 0.0, Stroke::new(1.0, Color32::DARK_GRAY));
            
            let max_kb = self.bandwidth_history.iter()
                .map(|(_, s, r)| s.max(*r))
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(1.0)
                .max(1.0);
            
            // Draw bandwidth lines
            for window in self.bandwidth_history.windows(2) {
                let (t1, s1, r1) = &window[0];
                let (t2, s2, r2) = &window[1];
                
                let x1 = chart_rect.left() + (t1.elapsed().as_secs_f32() / 60.0) * chart_rect.width();
                let x2 = chart_rect.left() + (t2.elapsed().as_secs_f32() / 60.0) * chart_rect.width();
                
                // Sent line (blue)
                let y1_sent = chart_rect.bottom() - (s1 / max_kb) * chart_rect.height();
                let y2_sent = chart_rect.bottom() - (s2 / max_kb) * chart_rect.height();
                painter.line_segment(
                    [Pos2::new(x1, y1_sent), Pos2::new(x2, y2_sent)],
                    Stroke::new(1.0, Color32::from_rgb(100, 150, 255)),
                );
                
                // Received line (green)
                let y1_recv = chart_rect.bottom() - (r1 / max_kb) * chart_rect.height();
                let y2_recv = chart_rect.bottom() - (r2 / max_kb) * chart_rect.height();
                painter.line_segment(
                    [Pos2::new(x1, y1_recv), Pos2::new(x2, y2_recv)],
                    Stroke::new(1.0, Color32::from_rgb(100, 255, 150)),
                );
            }
        }
    }
}