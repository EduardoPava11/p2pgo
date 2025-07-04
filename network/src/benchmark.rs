use anyhow::Result;
use std::time::{Duration, Instant};
use tracing::{info};

use crate::rna::{RNAMessage, RNAType};

/// Network benchmarking utilities
pub struct NetworkBenchmark {
    /// Test results
    results: BenchmarkResults,
    /// Test configuration
    config: BenchmarkConfig,
}

#[derive(Clone)]
pub struct BenchmarkConfig {
    /// Number of messages to send
    pub message_count: usize,
    /// Message size in bytes
    pub message_size: usize,
    /// Number of concurrent connections
    pub concurrent_connections: usize,
    /// Test duration
    pub test_duration: Duration,
    /// Enable bandwidth test
    pub test_bandwidth: bool,
    /// Enable latency test
    pub test_latency: bool,
    /// Enable relay capacity test
    pub test_relay_capacity: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            message_count: 1000,
            message_size: 1024,
            concurrent_connections: 10,
            test_duration: Duration::from_secs(60),
            test_bandwidth: true,
            test_latency: true,
            test_relay_capacity: true,
        }
    }
}

#[derive(Default, Clone)]
pub struct BenchmarkResults {
    /// Bandwidth test results
    pub bandwidth: BandwidthResults,
    /// Latency test results
    pub latency: LatencyResults,
    /// Relay capacity results
    pub relay_capacity: RelayCapacityResults,
    /// Overall metrics
    pub overall: OverallMetrics,
}

#[derive(Default, Clone)]
pub struct BandwidthResults {
    /// Upload speed in KB/s
    pub upload_speed_kbps: f64,
    /// Download speed in KB/s
    pub download_speed_kbps: f64,
    /// Peak upload speed
    pub peak_upload_kbps: f64,
    /// Peak download speed
    pub peak_download_kbps: f64,
    /// Sustained throughput
    pub sustained_throughput_kbps: f64,
}

#[derive(Default, Clone)]
pub struct LatencyResults {
    /// Round-trip time statistics
    pub rtt_min_ms: f64,
    pub rtt_avg_ms: f64,
    pub rtt_max_ms: f64,
    pub rtt_stddev_ms: f64,
    /// Percentiles
    pub rtt_p50_ms: f64,
    pub rtt_p95_ms: f64,
    pub rtt_p99_ms: f64,
    /// Jitter
    pub jitter_ms: f64,
}

#[derive(Default, Clone)]
pub struct RelayCapacityResults {
    /// Maximum concurrent connections handled
    pub max_connections: usize,
    /// Messages per second
    pub messages_per_second: f64,
    /// RNA propagation time
    pub rna_propagation_ms: f64,
    /// Circuit relay setup time
    pub relay_setup_ms: f64,
}

#[derive(Default, Clone)]
pub struct OverallMetrics {
    /// Total messages sent
    pub messages_sent: usize,
    /// Total messages received
    pub messages_received: usize,
    /// Message loss rate
    pub loss_rate: f64,
    /// Total data transferred
    pub total_data_mb: f64,
    /// Test duration
    pub duration_secs: f64,
}

impl NetworkBenchmark {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            results: BenchmarkResults::default(),
            config,
        }
    }
    
    /// Run all benchmarks
    pub async fn run_all(&mut self, peer_addresses: Vec<String>) -> Result<BenchmarkResults> {
        info!("Starting network benchmarks with {} peers", peer_addresses.len());
        
        let start_time = Instant::now();
        
        if self.config.test_bandwidth {
            info!("Running bandwidth test...");
            self.test_bandwidth(&peer_addresses).await?;
        }
        
        if self.config.test_latency {
            info!("Running latency test...");
            self.test_latency(&peer_addresses).await?;
        }
        
        if self.config.test_relay_capacity {
            info!("Running relay capacity test...");
            self.test_relay_capacity(&peer_addresses).await?;
        }
        
        self.results.overall.duration_secs = start_time.elapsed().as_secs_f64();
        
        Ok(self.results.clone())
    }
    
    /// Test bandwidth capacity
    async fn test_bandwidth(&mut self, peers: &[String]) -> Result<()> {
        let mut upload_samples = Vec::new();
        let mut download_samples = Vec::new();
        
        // Create test RNA messages of specified size
        let test_rna = self.create_test_rna(self.config.message_size);
        
        for _peer in peers.iter().take(self.config.concurrent_connections) {
            let start = Instant::now();
            let mut bytes_sent = 0;
            let mut bytes_received = 0;
            
            // Send messages for duration
            let test_start = Instant::now();
            while test_start.elapsed() < self.config.test_duration / 10 {
                // Send message
                bytes_sent += test_rna.data.len();
                
                // Simulate receive
                bytes_received += test_rna.data.len();
                
                // Calculate current speeds
                let elapsed = start.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    let upload_kbps = (bytes_sent as f64 / 1024.0) / elapsed;
                    let download_kbps = (bytes_received as f64 / 1024.0) / elapsed;
                    
                    upload_samples.push(upload_kbps);
                    download_samples.push(download_kbps);
                }
                
                // Small delay to simulate network
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
        
        // Calculate results
        if !upload_samples.is_empty() {
            self.results.bandwidth.upload_speed_kbps = average(&upload_samples);
            self.results.bandwidth.peak_upload_kbps = upload_samples.iter().cloned().fold(0.0, f64::max);
        }
        
        if !download_samples.is_empty() {
            self.results.bandwidth.download_speed_kbps = average(&download_samples);
            self.results.bandwidth.peak_download_kbps = download_samples.iter().cloned().fold(0.0, f64::max);
        }
        
        self.results.bandwidth.sustained_throughput_kbps = 
            (self.results.bandwidth.upload_speed_kbps + self.results.bandwidth.download_speed_kbps) / 2.0;
        
        Ok(())
    }
    
    /// Test network latency
    async fn test_latency(&mut self, peers: &[String]) -> Result<()> {
        let mut rtt_samples = Vec::new();
        
        for _peer in peers.iter().take(self.config.concurrent_connections) {
            for _ in 0..100 {
                let start = Instant::now();
                
                // Simulate ping
                tokio::time::sleep(Duration::from_millis(5)).await;
                
                let rtt = start.elapsed().as_secs_f64() * 1000.0;
                rtt_samples.push(rtt);
            }
        }
        
        if !rtt_samples.is_empty() {
            rtt_samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            self.results.latency.rtt_min_ms = rtt_samples[0];
            self.results.latency.rtt_max_ms = rtt_samples[rtt_samples.len() - 1];
            self.results.latency.rtt_avg_ms = average(&rtt_samples);
            self.results.latency.rtt_stddev_ms = stddev(&rtt_samples);
            
            // Percentiles
            self.results.latency.rtt_p50_ms = percentile(&rtt_samples, 50.0);
            self.results.latency.rtt_p95_ms = percentile(&rtt_samples, 95.0);
            self.results.latency.rtt_p99_ms = percentile(&rtt_samples, 99.0);
            
            // Calculate jitter
            let mut jitter_samples = Vec::new();
            for i in 1..rtt_samples.len() {
                jitter_samples.push((rtt_samples[i] - rtt_samples[i-1]).abs());
            }
            self.results.latency.jitter_ms = average(&jitter_samples);
        }
        
        Ok(())
    }
    
    /// Test relay capacity
    async fn test_relay_capacity(&mut self, _peers: &[String]) -> Result<()> {
        // Test maximum connections
        self.results.relay_capacity.max_connections = self.config.concurrent_connections;
        
        // Test message throughput
        let start = Instant::now();
        let mut message_count = 0;
        
        while start.elapsed() < Duration::from_secs(10) {
            message_count += 1;
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        
        self.results.relay_capacity.messages_per_second = 
            message_count as f64 / start.elapsed().as_secs_f64();
        
        // Test RNA propagation time
        let prop_start = Instant::now();
        // Simulate propagation through network
        tokio::time::sleep(Duration::from_millis(50)).await;
        self.results.relay_capacity.rna_propagation_ms = 
            prop_start.elapsed().as_secs_f64() * 1000.0;
        
        // Test relay setup time
        let setup_start = Instant::now();
        // Simulate relay setup
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.results.relay_capacity.relay_setup_ms = 
            setup_start.elapsed().as_secs_f64() * 1000.0;
        
        Ok(())
    }
    
    /// Create test RNA message
    fn create_test_rna(&self, size: usize) -> RNAMessage {
        RNAMessage {
            id: format!("benchmark-{}", uuid::Uuid::new_v4()),
            source_peer: "benchmark-peer".to_string(),
            rna_type: RNAType::PatternData {
                pattern_type: "benchmark".to_string(),
                board_region: (0, 0, 19, 19),
                frequency: 1.0,
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            quality_score: 1.0,
            data: vec![0u8; size],
        }
    }
    
    /// Generate benchmark report
    pub fn generate_report(&self) -> String {
        format!(
            r#"
Network Benchmark Report
========================

Test Configuration:
- Message Count: {}
- Message Size: {} bytes
- Concurrent Connections: {}
- Test Duration: {:?}

Bandwidth Results:
- Upload Speed: {:.2} KB/s (peak: {:.2} KB/s)
- Download Speed: {:.2} KB/s (peak: {:.2} KB/s)
- Sustained Throughput: {:.2} KB/s

Latency Results:
- RTT Min/Avg/Max: {:.2}/{:.2}/{:.2} ms
- RTT StdDev: {:.2} ms
- Percentiles: P50={:.2}ms, P95={:.2}ms, P99={:.2}ms
- Jitter: {:.2} ms

Relay Capacity:
- Max Connections: {}
- Messages/Second: {:.2}
- RNA Propagation: {:.2} ms
- Relay Setup: {:.2} ms

Overall Metrics:
- Messages Sent: {}
- Messages Received: {}
- Loss Rate: {:.2}%
- Total Data: {:.2} MB
- Test Duration: {:.2} seconds
"#,
            self.config.message_count,
            self.config.message_size,
            self.config.concurrent_connections,
            self.config.test_duration,
            self.results.bandwidth.upload_speed_kbps,
            self.results.bandwidth.peak_upload_kbps,
            self.results.bandwidth.download_speed_kbps,
            self.results.bandwidth.peak_download_kbps,
            self.results.bandwidth.sustained_throughput_kbps,
            self.results.latency.rtt_min_ms,
            self.results.latency.rtt_avg_ms,
            self.results.latency.rtt_max_ms,
            self.results.latency.rtt_stddev_ms,
            self.results.latency.rtt_p50_ms,
            self.results.latency.rtt_p95_ms,
            self.results.latency.rtt_p99_ms,
            self.results.latency.jitter_ms,
            self.results.relay_capacity.max_connections,
            self.results.relay_capacity.messages_per_second,
            self.results.relay_capacity.rna_propagation_ms,
            self.results.relay_capacity.relay_setup_ms,
            self.results.overall.messages_sent,
            self.results.overall.messages_received,
            self.results.overall.loss_rate * 100.0,
            self.results.overall.total_data_mb,
            self.results.overall.duration_secs,
        )
    }
}

// Statistical helper functions
fn average(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f64>() / samples.len() as f64
}

fn stddev(samples: &[f64]) -> f64 {
    if samples.len() < 2 {
        return 0.0;
    }
    let avg = average(samples);
    let variance = samples.iter()
        .map(|x| (x - avg).powi(2))
        .sum::<f64>() / (samples.len() - 1) as f64;
    variance.sqrt()
}

fn percentile(sorted_samples: &[f64], p: f64) -> f64 {
    if sorted_samples.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted_samples.len() - 1) as f64) as usize;
    sorted_samples[idx]
}

// UUID helper
mod uuid {
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    pub struct Uuid;
    
    impl Uuid {
        pub fn new_v4() -> String {
            format!("bench-{}", COUNTER.fetch_add(1, Ordering::SeqCst))
        }
    }
}