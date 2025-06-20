// SPDX-License-Identifier: MIT OR Apache-2.0

//! Crash logging with rotation for macOS

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs;
use tokio::sync::Mutex;
use anyhow::Result;
use tracing::info;

/// Crash logger with 1GB rotation
pub struct CrashLogger {
    log_dir: PathBuf,
    current_size: AtomicU64,
    max_size: u64,
    mutex: Mutex<()>,
}

impl CrashLogger {
    /// Create a new crash logger
    pub fn new() -> Result<Self> {
        let log_dir = Self::get_log_directory()?;
        
        Ok(Self {
            log_dir,
            current_size: AtomicU64::new(0),
            max_size: 1024 * 1024 * 1024, // 1GB
            mutex: Mutex::new(()),
        })
    }
    
    /// Get the macOS Logs directory
    fn get_log_directory() -> Result<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
            let mut path = PathBuf::from(home);
            path.push("Library");
            path.push("Logs");
            path.push("p2pgo");
            Ok(path)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            // Fallback for other platforms
            let mut path = PathBuf::from(".");
            path.push("logs");
            Ok(path)
        }
    }
    
    /// Initialize the logger and calculate current size
    pub async fn init(&self) -> Result<()> {
        // Ensure log directory exists
        fs::create_dir_all(&self.log_dir).await?;
        
        // Calculate current log size
        let mut total_size = 0u64;
        let mut entries = fs::read_dir(&self.log_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                total_size += metadata.len();
            }
        }
        
        self.current_size.store(total_size, Ordering::Relaxed);
        info!("Crash logger initialized. Current log size: {} bytes", total_size);
        
        Ok(())
    }
    
    /// Log a crash with rotation check
    pub async fn log_crash(&self, error: &str, context: &str) -> Result<()> {
        let _guard = self.mutex.lock().await;
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("crash_{}.log", timestamp);
        let file_path = self.log_dir.join(filename);
        
        let log_entry = format!(
            "[{}] CRASH: {}\nContext: {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            error,
            context
        );
        
        // Write the crash log
        fs::write(&file_path, log_entry.as_bytes()).await?;
        
        // Update size tracking
        let new_size = log_entry.len() as u64;
        let total_size = self.current_size.fetch_add(new_size, Ordering::Relaxed) + new_size;
        
        info!("Crash logged to {:?}. Total log size: {} bytes", file_path, total_size);
        
        // Check if rotation is needed
        if total_size > self.max_size {
            self.rotate_logs().await?;
        }
        
        Ok(())
    }
    
    /// Rotate logs when they exceed 1GB
    async fn rotate_logs(&self) -> Result<()> {
        info!("Starting log rotation...");
        
        let mut entries = fs::read_dir(&self.log_dir).await?;
        let mut log_files = Vec::new();
        
        // Collect all log files with their metadata
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                if let Ok(metadata) = entry.metadata().await {
                    log_files.push((path, metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)));
                }
            }
        }
        
        // Sort by modification time (oldest first)
        log_files.sort_by_key(|(_, time)| *time);
        
        // Remove oldest 50% of files
        let remove_count = log_files.len() / 2;
        let mut removed_size = 0u64;
        
        for (path, _) in log_files.iter().take(remove_count) {
            if let Ok(metadata) = fs::metadata(path).await {
                removed_size += metadata.len();
            }
            
            if let Err(e) = fs::remove_file(path).await {
                tracing::warn!("Failed to remove old log file {:?}: {}", path, e);
            } else {
                tracing::debug!("Removed old log file: {:?}", path);
            }
        }
        
        // Update size tracking
        self.current_size.fetch_sub(removed_size, Ordering::Relaxed);
        
        info!(
            "Log rotation completed. Removed {} files, {} bytes",
            remove_count, removed_size
        );
        
        Ok(())
    }
    
    /// Get current log directory size
    pub fn get_current_size(&self) -> u64 {
        self.current_size.load(Ordering::Relaxed)
    }
    
    /// Get maximum log size before rotation
    pub fn get_max_size(&self) -> u64 {
        self.max_size
    }
}

/// Global crash logger instance
use std::sync::OnceLock;
static CRASH_LOGGER: OnceLock<CrashLogger> = OnceLock::new();

/// Initialize the global crash logger
pub async fn init_crash_logger() -> Result<()> {
    let logger = CrashLogger::new()?;
    logger.init().await?;
    
    CRASH_LOGGER.set(logger).map_err(|_| anyhow::anyhow!("Crash logger already initialized"))?;
    
    Ok(())
}

/// Log a crash using the global logger
pub async fn log_crash(error: &str, context: &str) -> Result<()> {
    if let Some(logger) = CRASH_LOGGER.get() {
        logger.log_crash(error, context).await
    } else {
        Err(anyhow::anyhow!("Crash logger not initialized"))
    }
}

/// Get crash logger statistics
pub fn get_crash_logger_stats() -> Option<(u64, u64)> {
    CRASH_LOGGER.get().map(|logger| (logger.get_current_size(), logger.get_max_size()))
}
