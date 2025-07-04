// SPDX-License-Identifier: MIT OR Apache-2.0

//! Network utilities and helper functions for P2P Go

#![deny(warnings)]

use anyhow::{Result, anyhow};
use std::sync::{atomic::{AtomicU32, Ordering}, Arc};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::task::JoinHandle;

// Global restart counters for each task type - used for telemetry and restart policy
static TASK_RESTARTS: std::sync::OnceLock<std::sync::Mutex<HashMap<String, AtomicU32>>> = std::sync::OnceLock::new();

/// Token for signaling cancelation of a task
#[derive(Debug, Clone)]
pub struct CancellationToken {
    inner: Arc<tokio::sync::Notify>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationToken {
    /// Create a new cancellation token
    pub fn new() -> Self {
        Self {
            inner: Arc::new(tokio::sync::Notify::new()),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Check if the token has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
    
    /// Cancel the token, notifying all waiters
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.inner.notify_waiters();
    }
    
    /// Wait for cancellation
    pub async fn cancelled(&self) {
        if !self.is_cancelled() {
            self.inner.notified().await;
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// A robust cancelable task with automatic restart capability
pub struct CancelableTask {
    pub name: String,
    pub token: CancellationToken,
    pub handle: Option<JoinHandle<Result<()>>>,
    pub restart_fn: Box<dyn Fn() -> JoinHandle<Result<()>> + Send + Sync>,
}

impl CancelableTask {
    /// Manually restart the task
    pub async fn restart(&mut self) -> Result<()> {
        // Cancel the current task
        self.token.cancel();
        
        // Wait for the handle to complete
        if let Some(handle) = self.handle.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }
        
        // Create a new token
        self.token = CancellationToken::new();
        
        // Start a new task
        let restart_fn = &self.restart_fn;
        self.handle = Some(restart_fn());
        
        tracing::info!("Task '{}' restarted manually", self.name);
        
        Ok(())
    }
    
    /// Cancel the task and wait for it to complete
    pub async fn cancel(&mut self) -> Result<()> {
        // Cancel the current task
        self.token.cancel();
        
        // Wait for the handle to complete
        if let Some(handle) = self.handle.take() {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(result) => {
                    if let Err(e) = result {
                        if e.is_cancelled() {
                            // Task was cancelled, this is expected
                            return Ok(());
                        } else if e.is_panic() {
                            return Err(anyhow!("Task '{}' panicked during shutdown", self.name));
                        }
                    }
                },
                Err(_) => {
                    return Err(anyhow!("Task '{}' did not shut down within timeout", self.name));
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if the task is still running
    pub fn is_running(&self) -> bool {
        self.handle.as_ref().map_or(false, |h| !h.is_finished())
    }
}

impl Drop for CancelableTask {
    fn drop(&mut self) {
        self.token.cancel();
        // We don't wait for the task to complete here since Drop is not async
    }
}

/// Spawn a cancelable task with panic recovery and automatic restart
///
/// This macro wraps tokio::spawn with additional error handling:
/// - Catches panics and restarts the task gracefully
/// - Limits restarts to avoid infinite crash loops
/// - Tracks task health and statistics
///
/// # Examples
///
/// ```ignore
/// // This is a usage example, not a runnable test
/// let task = spawn_cancelable!(
///     name: "my_background_task",
///     max_restarts: 3,
///     restart_delay_ms: 2000,
///     window_secs: 30,
///     |shutdown| async move {
///         // Your long-running task code here
///         // Check shutdown.is_cancelled() to handle graceful termination
///     }
/// );
///
/// // To cancel and wait for the task:
/// task.cancel().await;
/// ```
#[macro_export]
macro_rules! spawn_cancelable {
    (
        name: $name:expr,
        max_restarts: $max_restarts:expr,
        restart_delay_ms: $restart_delay:expr,
        window_secs: $window_secs:expr,
        |$shutdown:ident| $body:expr
    ) => {{
        use std::sync::Arc;
        use std::time::{Duration, Instant};
        use tokio::task::JoinHandle;
        use $crate::net_util::{CancelableTask, CancellationToken};
        
        let task_name = $name.to_string();
        let task_token = CancellationToken::new();
        let window_duration = std::time::Duration::from_secs($window_secs);
        let restart_delay = std::time::Duration::from_millis($restart_delay);
        let restart_counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let restart_window_start = Arc::new(std::sync::atomic::AtomicU64::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::from_secs(0))
                .as_secs()
        ));
        
        let run_task = {
            let task_token = task_token.clone();
            let task_name = task_name.clone();
            let restart_counter = restart_counter.clone();
            let restart_window_start = restart_window_start.clone();
            
            move || {
                let task_name = task_name.clone();
                let task_token = task_token.clone();
                let restart_counter = restart_counter.clone();
                let restart_window_start = restart_window_start.clone();
                
                tokio::spawn(async move {
                    let mut consecutive_failures = 0;
                    
                    loop {
                        // Reset restart counter if window has elapsed
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_else(|_| Duration::from_secs(0))
                            .as_secs();
                        
                        let window_start = restart_window_start.load(std::sync::atomic::Ordering::Relaxed);
                        if now > window_start + window_duration.as_secs() {
                            restart_counter.store(0, std::sync::atomic::Ordering::Relaxed);
                            restart_window_start.store(now, std::sync::atomic::Ordering::Relaxed);
                        }
                        
                        // Check if we've exceeded max restarts
                        let restarts = restart_counter.load(std::sync::atomic::Ordering::Relaxed);
                        if restarts > $max_restarts {
                            tracing::error!("Task '{}' failed too many times ({}) within window, giving up", 
                                task_name, restarts);
                            break;
                        }
                        
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let $shutdown = task_token.clone();
                            
                            // Run the actual task
                            let future = $body;
                            futures_lite::future::block_on(future)
                        }));
                        
                        // Handle the result or panic
                        match result {
                            Ok(Ok(_)) => {
                                // Task completed successfully
                                tracing::debug!("Task '{}' completed successfully", task_name);
                                break;
                            },
                            Ok(Err(e)) => {
                                // Task returned an error
                                tracing::error!("Task '{}' failed with error: {}", task_name, e);
                                consecutive_failures += 1;
                            },
                            Err(e) => {
                                // Task panicked
                                let panic_msg = if let Some(s) = e.downcast_ref::<String>() {
                                    s.clone()
                                } else if let Some(s) = e.downcast_ref::<&str>() {
                                    s.to_string()
                                } else {
                                    "Unknown panic".to_string()
                                };
                                
                                tracing::error!("Task '{}' panicked: {}", task_name, panic_msg);
                                consecutive_failures += 1;
                            }
                        }
                        
                        // Check if task was cancelled before restarting
                        if task_token.is_cancelled() {
                            tracing::info!("Task '{}' cancelled, not restarting", task_name);
                            break;
                        }
                        
                        // Increment restart counter
                        restart_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        
                        // Also update global restart counter for telemetry
                        $crate::net_util::increment_restart_counter(&task_name);
                        
                        // Wait before restarting to avoid rapid restart loops
                        tracing::info!("Restarting task '{}' in {}ms (attempt {})", 
                            task_name, restart_delay.as_millis(), consecutive_failures);
                        tokio::time::sleep(restart_delay).await;
                    }
                    
                    Ok::<(), anyhow::Error>(())
                })
            }
        };
        
        // Start the task initially
        let handle = run_task();
        
        CancelableTask {
            name: task_name,
            token: task_token,
            handle: Some(handle),
            restart_fn: Box::new(run_task),
        }
    }};
    
    // Legacy API for compatibility
    ($task_type:expr, $future:expr, $err_handler:expr) => {
        {
            // Register task type in global counter if not already done
            let restarts = $crate::net_util::get_restart_counters();
            let mut counters = restarts.lock().unwrap();
            
            if !counters.contains_key($task_type) {
                counters.insert($task_type.to_string(), std::sync::atomic::AtomicU32::new(0));
            }
            drop(counters);
            
            // Clone values for the closure
            let task_type = $task_type.to_string();
            
            tokio::spawn(async move {
                // First attempt
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    Box::pin($future)
                })) {
                    Ok(fut) => match fut.await {
                        Ok(result) => {
                            return result;
                        }
                        Err(e) => {
                            // Task returned an error
                            let restart = $err_handler(e);
                            if restart {
                                $crate::net_util::increment_restart_counter(&task_type);
                                
                                // Try once more
                                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    Box::pin($future)
                                })) {
                                    Ok(fut2) => fut2.await,
                                    Err(panic) => {
                                        // Convert panic to anyhow error
                                        let panic_msg = if let Some(s) = panic.downcast_ref::<String>() {
                                            s.clone()
                                        } else if let Some(s) = panic.downcast_ref::<&str>() {
                                            s.to_string()
                                        } else {
                                            "Unknown panic".to_string()
                                        };
                                        Err(anyhow::anyhow!("Task panicked on retry: {}", panic_msg))
                                    }
                                }
                            } else {
                                Err(e)
                            }
                        }
                    },
                    Err(panic) => {
                        // Convert panic to anyhow error
                        let panic_msg = if let Some(s) = panic.downcast_ref::<String>() {
                            s.clone()
                        } else if let Some(s) = panic.downcast_ref::<&str>() {
                            s.to_string()
                        } else {
                            "Unknown panic".to_string()
                        };
                        let err = anyhow::anyhow!("Task panicked: {}", panic_msg);
                        
                        // Call error handler
                        let restart = $err_handler(err);
                        if restart {
                            $crate::net_util::increment_restart_counter(&task_type);
                            
                            // Try once more
                            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                Box::pin($future)
                            })) {
                                Ok(fut2) => fut2.await,
                                Err(panic2) => {
                                    // Convert panic to anyhow error
                                    let panic_msg2 = if let Some(s) = panic2.downcast_ref::<String>() {
                                        s.clone()
                                    } else if let Some(s) = panic2.downcast_ref::<&str>() {
                                        s.to_string()
                                    } else {
                                        "Unknown panic".to_string()
                                    };
                                    Err(anyhow::anyhow!("Task panicked on retry: {}", panic_msg2))
                                }
                            }
                        } else {
                            Err(anyhow::anyhow!("Task panicked and not restarted: {}", panic_msg))
                        }
                    }
                }
            })
        }
    };
}

/// Get the global restart counters HashMap
pub fn get_restart_counters() -> &'static std::sync::Mutex<HashMap<String, AtomicU32>> {
    TASK_RESTARTS.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

/// Increment the restart counter for a specific task type
pub fn increment_restart_counter(task_type: &str) -> u32 {
    let restarts = get_restart_counters();
    if let Ok(counters) = restarts.lock() {
        if let Some(counter) = counters.get(task_type) {
            counter.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            // Should never happen if spawn_cancelable is used correctly
            0
        }
    } else {
        // Lock poisoned, return 0
        0
    }
}

/// Get the restart count for a specific task type
pub fn get_restart_count(task_type: &str) -> u32 {
    let restarts = get_restart_counters();
    if let Ok(counters) = restarts.lock() {
        if let Some(counter) = counters.get(task_type) {
            counter.load(Ordering::SeqCst)
        } else {
            0
        }
    } else {
        // Lock poisoned, return 0
        0
    }
}

/// Find a free port pair (two consecutive ports) that can be used for networking
pub fn find_free_port_pair() -> Result<(u16, u16)> {
    // Try to find two consecutive available ports
    for _ in 0..10 {
        let port1 = crate::port::pick_available_port()?;
        let port2 = port1 + 1;
        
        if crate::port::is_port_available(port2) {
            return Ok((port1, port2));
        }
    }
    
    // If we couldn't find consecutive ports, just return two separate ports
    let port1 = crate::port::pick_available_port()?;
    let port2 = crate::port::pick_available_port()?;
    
    Ok((port1, port2))
}

/// Save port pair to a temporary file for reuse
pub fn save_port_pair(port1: u16, port2: u16) -> Result<()> {
    use std::io::Write;
    
    let path = crate::port::get_port_config_path()?;
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    
    // Only store the port pair if they're different
    if port1 == port2 {
        return Err(anyhow::anyhow!("Port pair contains identical ports"));
    }
    
    let now = SystemTime::now();
    let contents = format!(
        "port1 = {}\nport2 = {}\nlast_used = \"{}\"", 
        port1, 
        port2, 
        humantime::format_rfc3339(now)
    );
    
    // Write the file atomically
    let temp_path = path.with_extension("tmp");
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    
    // Rename the file to the final destination
    std::fs::rename(temp_path, path)?;
    
    Ok(())
}

/// Load a previously saved port pair
pub fn load_port_pair() -> Result<Option<(u16, u16)>> {
    use std::io::Read;
    
    let path = match crate::port::get_port_config_path() {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };
    
    if !path.exists() {
        return Ok(None);
    }
    
    let mut file = std::fs::File::open(&path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    // Parse the simple format
    let mut port1 = None;
    let mut port2 = None;
    
    for line in contents.lines() {
        if let Some(p) = line.strip_prefix("port1 = ") {
            port1 = Some(p.trim().parse::<u16>()?);
        } else if let Some(p) = line.strip_prefix("port2 = ") {
            port2 = Some(p.trim().parse::<u16>()?);
        }
    }
    
    match (port1, port2) {
        (Some(p1), Some(p2)) => {
            // Verify both ports are available
            if crate::port::is_port_available(p1) && crate::port::is_port_available(p2) {
                Ok(Some((p1, p2)))
            } else {
                Ok(None) // Ports not available
            }
        }
        _ => Ok(None), // Invalid or incomplete file
    }
}
