use crate::test_helpers::*;
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::info;

/// Simulated network visualization state
pub struct NetworkVisualizationState {
    pub relays: HashMap<String, RelayNode>,
    pub connections: Vec<Connection>,
    pub packets: Vec<PacketAnimation>,
    pub stats: NetworkStats,
}

pub struct RelayNode {
    pub id: String,
    pub position: (f32, f32),
    pub is_local: bool,
    pub discovery_score: f32,
    pub active_connections: usize,
}

pub struct Connection {
    pub from: String,
    pub to: String,
    pub connection_type: ConnectionType,
    pub latency_ms: f32,
    pub bandwidth_kbps: f32,
}

pub struct PacketAnimation {
    pub id: String,
    pub from: String,
    pub to: String,
    pub rna_type: RNAVisualizationType,
    pub size_kb: f32,
    pub progress: f32, // 0.0 to 1.0
    pub start_time: Instant,
}

#[derive(Clone, Copy)]
pub enum RNAVisualizationType {
    GameData,      // mRNA - full games
    PatternData,   // tRNA - patterns
    ModelWeights,  // Neural net updates
    Discovery,     // Network discovery
}

pub struct NetworkStats {
    pub packets_sent: usize,
    pub packets_received: usize,
    pub total_sent_kb: f32,
    pub total_received_kb: f32,
    pub active_connections: usize,
    pub discovery_score: f32,
}

#[tokio::test]
async fn test_network_visualization_update() -> Result<()> {
    let mut viz_state = NetworkVisualizationState {
        relays: HashMap::new(),
        connections: Vec::new(),
        packets: Vec::new(),
        stats: NetworkStats {
            packets_sent: 0,
            packets_received: 0,
            total_sent_kb: 0.0,
            total_received_kb: 0.0,
            active_connections: 0,
            discovery_score: 0.0,
        },
    };
    
    // Add relays
    viz_state.relays.insert("relay1".to_string(), RelayNode {
        id: "relay1".to_string(),
        position: (100.0, 100.0),
        is_local: true,
        discovery_score: 0.8,
        active_connections: 0,
    });
    
    viz_state.relays.insert("relay2".to_string(), RelayNode {
        id: "relay2".to_string(),
        position: (300.0, 100.0),
        is_local: false,
        discovery_score: 0.6,
        active_connections: 0,
    });
    
    viz_state.relays.insert("relay3".to_string(), RelayNode {
        id: "relay3".to_string(),
        position: (200.0, 250.0),
        is_local: false,
        discovery_score: 0.7,
        active_connections: 0,
    });
    
    // Add connections
    viz_state.connections.push(Connection {
        from: "relay1".to_string(),
        to: "relay2".to_string(),
        connection_type: ConnectionType::Direct,
        latency_ms: 5.0,
        bandwidth_kbps: 1000.0,
    });
    
    viz_state.connections.push(Connection {
        from: "relay1".to_string(),
        to: "relay3".to_string(),
        connection_type: ConnectionType::Relayed,
        latency_ms: 25.0,
        bandwidth_kbps: 500.0,
    });
    
    viz_state.connections.push(Connection {
        from: "relay2".to_string(),
        to: "relay3".to_string(),
        connection_type: ConnectionType::Direct,
        latency_ms: 10.0,
        bandwidth_kbps: 800.0,
    });
    
    // Update connection counts
    for conn in &viz_state.connections {
        if let Some(relay) = viz_state.relays.get_mut(&conn.from) {
            relay.active_connections += 1;
        }
        if let Some(relay) = viz_state.relays.get_mut(&conn.to) {
            relay.active_connections += 1;
        }
    }
    
    viz_state.stats.active_connections = viz_state.connections.len();
    
    // Simulate packet animations
    let packet_types = vec![
        (RNAVisualizationType::GameData, 150.0),
        (RNAVisualizationType::PatternData, 50.0),
        (RNAVisualizationType::ModelWeights, 200.0),
        (RNAVisualizationType::Discovery, 10.0),
    ];
    
    for (i, (rna_type, size_kb)) in packet_types.iter().enumerate() {
        viz_state.packets.push(PacketAnimation {
            id: format!("packet-{}", i),
            from: "relay1".to_string(),
            to: if i % 2 == 0 { "relay2" } else { "relay3" }.to_string(),
            rna_type: *rna_type,
            size_kb: *size_kb,
            progress: 0.0,
            start_time: Instant::now(),
        });
        
        viz_state.stats.packets_sent += 1;
        viz_state.stats.total_sent_kb += size_kb;
    }
    
    // Simulate packet movement
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Update packet positions
        for packet in &mut viz_state.packets {
            packet.progress = (packet.progress + 0.1).min(1.0);
        }
        
        // Remove completed packets
        viz_state.packets.retain(|p| {
            if p.progress >= 1.0 {
                viz_state.stats.packets_received += 1;
                viz_state.stats.total_received_kb += p.size_kb;
                false
            } else {
                true
            }
        });
        
        info!("Active packets: {}, Total sent: {:.1} KB", 
            viz_state.packets.len(), 
            viz_state.stats.total_sent_kb
        );
    }
    
    // Verify visualization state
    assert_eq!(viz_state.relays.len(), 3);
    assert_eq!(viz_state.connections.len(), 3);
    assert_eq!(viz_state.stats.packets_sent, 4);
    assert_eq!(viz_state.stats.packets_received, 4);
    assert!(viz_state.stats.total_sent_kb > 400.0);
    
    Ok(())
}

#[tokio::test]
async fn test_bandwidth_graph_data() -> Result<()> {
    // Simulate bandwidth data collection over time
    let mut bandwidth_history = BandwidthHistory {
        upload_history: Vec::new(),
        download_history: Vec::new(),
        max_samples: 60, // 1 minute of data at 1 sample/sec
    };
    
    // Simulate varying bandwidth usage
    for i in 0..60 {
        let time = i as f32;
        
        // Simulate upload spikes every 10 seconds
        let upload_kbps = if i % 10 < 3 {
            500.0 + (i % 10) as f32 * 100.0
        } else {
            50.0 + (i % 5) as f32 * 10.0
        };
        
        // Simulate steady download with occasional drops
        let download_kbps = if i % 15 == 0 {
            20.0
        } else {
            200.0 + (i % 7) as f32 * 20.0
        };
        
        bandwidth_history.add_sample(time, upload_kbps, download_kbps);
        
        if i % 10 == 0 {
            info!("Time {}s - Upload: {:.1} kbps, Download: {:.1} kbps", 
                time, upload_kbps, download_kbps);
        }
    }
    
    // Verify data collection
    assert_eq!(bandwidth_history.upload_history.len(), 60);
    assert_eq!(bandwidth_history.download_history.len(), 60);
    
    // Calculate statistics
    let avg_upload = bandwidth_history.average_upload();
    let avg_download = bandwidth_history.average_download();
    let peak_upload = bandwidth_history.peak_upload();
    let peak_download = bandwidth_history.peak_download();
    
    info!("Bandwidth stats - Avg Upload: {:.1} kbps, Peak Upload: {:.1} kbps", 
        avg_upload, peak_upload);
    info!("Bandwidth stats - Avg Download: {:.1} kbps, Peak Download: {:.1} kbps", 
        avg_download, peak_download);
    
    assert!(avg_upload > 100.0 && avg_upload < 300.0);
    assert!(avg_download > 150.0 && avg_download < 250.0);
    assert!(peak_upload > 600.0);
    assert!(peak_download > 300.0);
    
    Ok(())
}

struct BandwidthHistory {
    upload_history: Vec<(f32, f32)>, // (time, kbps)
    download_history: Vec<(f32, f32)>,
    max_samples: usize,
}

impl BandwidthHistory {
    fn add_sample(&mut self, time: f32, upload_kbps: f32, download_kbps: f32) {
        self.upload_history.push((time, upload_kbps));
        self.download_history.push((time, download_kbps));
        
        if self.upload_history.len() > self.max_samples {
            self.upload_history.remove(0);
        }
        if self.download_history.len() > self.max_samples {
            self.download_history.remove(0);
        }
    }
    
    fn average_upload(&self) -> f32 {
        if self.upload_history.is_empty() {
            return 0.0;
        }
        self.upload_history.iter().map(|(_, kbps)| kbps).sum::<f32>() 
            / self.upload_history.len() as f32
    }
    
    fn average_download(&self) -> f32 {
        if self.download_history.is_empty() {
            return 0.0;
        }
        self.download_history.iter().map(|(_, kbps)| kbps).sum::<f32>() 
            / self.download_history.len() as f32
    }
    
    fn peak_upload(&self) -> f32 {
        self.upload_history.iter()
            .map(|(_, kbps)| *kbps)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }
    
    fn peak_download(&self) -> f32 {
        self.download_history.iter()
            .map(|(_, kbps)| *kbps)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }
}

#[tokio::test]
async fn test_connection_quality_indicators() -> Result<()> {
    // Test connection quality visualization
    let connections = vec![
        ("relay1", "relay2", 5.0, 1000.0, 0.0),    // Excellent
        ("relay1", "relay3", 50.0, 500.0, 0.1),    // Good
        ("relay2", "relay4", 150.0, 200.0, 0.5),   // Fair
        ("relay3", "relay5", 300.0, 50.0, 2.0),    // Poor
    ];
    
    for (from, to, latency_ms, bandwidth_kbps, packet_loss) in connections {
        let quality = calculate_connection_quality(latency_ms, bandwidth_kbps, packet_loss);
        let (color, label) = get_quality_indicator(quality);
        
        info!("Connection {}->{}: latency={}ms, bandwidth={}kbps, loss={}%", 
            from, to, latency_ms, bandwidth_kbps, packet_loss * 100.0);
        info!("  Quality: {:.2} - {} ({})", quality, label, color);
        
        match label {
            "Excellent" => assert!(quality > 0.8),
            "Good" => assert!(quality > 0.6 && quality <= 0.8),
            "Fair" => assert!(quality > 0.4 && quality <= 0.6),
            "Poor" => assert!(quality <= 0.4),
            _ => panic!("Unknown quality label"),
        }
    }
    
    Ok(())
}

fn calculate_connection_quality(latency_ms: f32, bandwidth_kbps: f32, packet_loss: f32) -> f32 {
    let latency_score = 1.0 - (latency_ms / 500.0).min(1.0);
    let bandwidth_score = (bandwidth_kbps / 1000.0).min(1.0);
    let loss_score = 1.0 - packet_loss.min(1.0);
    
    // Weighted average
    (latency_score * 0.4 + bandwidth_score * 0.4 + loss_score * 0.2).max(0.0).min(1.0)
}

fn get_quality_indicator(quality: f32) -> (&'static str, &'static str) {
    match quality {
        q if q > 0.8 => ("Green", "Excellent"),
        q if q > 0.6 => ("Yellow", "Good"),
        q if q > 0.4 => ("Orange", "Fair"),
        _ => ("Red", "Poor"),
    }
}

#[tokio::test]
async fn test_discovery_animation() -> Result<()> {
    // Test discovery pulse animation
    let mut discovery_pulses = Vec::new();
    
    // Create discovery pulses from different relays
    for i in 0..3 {
        discovery_pulses.push(DiscoveryPulse {
            source: format!("relay{}", i + 1),
            radius: 0.0,
            max_radius: 200.0,
            alpha: 1.0,
            discovery_score_boost: 0.1 * (i + 1) as f32,
        });
    }
    
    // Animate pulses
    for frame in 0..20 {
        for pulse in &mut discovery_pulses {
            pulse.update(0.05); // 50ms per frame
        }
        
        if frame % 5 == 0 {
            info!("Frame {}: Pulse radii: {:?}", 
                frame,
                discovery_pulses.iter().map(|p| p.radius as i32).collect::<Vec<_>>()
            );
        }
    }
    
    // All pulses should have completed
    for pulse in &discovery_pulses {
        assert!(pulse.radius >= pulse.max_radius);
        assert!(pulse.alpha <= 0.1);
    }
    
    Ok(())
}

struct DiscoveryPulse {
    source: String,
    radius: f32,
    max_radius: f32,
    alpha: f32,
    discovery_score_boost: f32,
}

impl DiscoveryPulse {
    fn update(&mut self, dt: f32) {
        self.radius += 100.0 * dt; // Expand at 100 pixels/second
        self.alpha = (1.0 - self.radius / self.max_radius).max(0.0);
    }
}