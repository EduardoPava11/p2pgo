// SPDX-License-Identifier: MIT OR Apache-2.0

//! External terminal log streaming functionality

use anyhow::Result;
use std::process::{Command, Stdio};
use tokio::sync::broadcast;

/// Log streaming manager for external terminal viewers
pub struct LogStreamer {
    sender: broadcast::Sender<String>,
}

impl LogStreamer {
    /// Create a new log streamer
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    /// Spawn external terminal log viewer
    pub async fn spawn_external_viewer(&self) -> Result<()> {
        let _span =
            tracing::info_span!("cli.log_streamer", "LogStreamer::spawn_external_viewer").entered();

        // Create a log receiver for the external viewer
        let mut receiver = self.sender.subscribe();

        // Spawn a new terminal window with a log viewer
        let viewer_command = self.get_terminal_command();

        tracing::info!(
            command = ?viewer_command,
            "Spawning external log viewer"
        );

        let mut child = Command::new(&viewer_command[0])
            .args(&viewer_command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn terminal: {}", e))?;

        // Stream logs to the external viewer
        tokio::spawn(async move {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;

                while let Ok(log_entry) = receiver.recv().await {
                    let line = format!("{}\n", log_entry);
                    if stdin.write_all(line.as_bytes()).is_err() {
                        break;
                    }
                    if stdin.flush().is_err() {
                        break;
                    }
                }
            }

            // Wait for child to finish
            let _ = child.wait();
        });

        Ok(())
    }

    /// Send a log entry to external viewers
    pub fn log(&self, entry: &str) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
        let formatted = format!("[{}] {}", timestamp, entry);
        let _ = self.sender.send(formatted);
    }

    /// Get platform-specific terminal command
    fn get_terminal_command(&self) -> Vec<String> {
        #[cfg(target_os = "macos")]
        {
            vec![
                "osascript".to_string(),
                "-e".to_string(),
                "tell application \"Terminal\" to do script \"tail -f /dev/stdin | grep --line-buffered .\"".to_string(),
            ]
        }

        #[cfg(target_os = "linux")]
        {
            // Try common terminal emulators
            if Command::new("gnome-terminal")
                .arg("--version")
                .output()
                .is_ok()
            {
                vec![
                    "gnome-terminal".to_string(),
                    "--".to_string(),
                    "bash".to_string(),
                    "-c".to_string(),
                    "tail -f /dev/stdin".to_string(),
                ]
            } else if Command::new("xterm").arg("-version").output().is_ok() {
                vec![
                    "xterm".to_string(),
                    "-e".to_string(),
                    "tail -f /dev/stdin".to_string(),
                ]
            } else {
                vec![
                    "x-terminal-emulator".to_string(),
                    "-e".to_string(),
                    "tail -f /dev/stdin".to_string(),
                ]
            }
        }

        #[cfg(target_os = "windows")]
        {
            vec![
                "cmd".to_string(),
                "/c".to_string(),
                "start".to_string(),
                "powershell".to_string(),
                "-Command".to_string(),
                "Get-Content -Path - -Wait".to_string(),
            ]
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            // Fallback
            vec![
                "echo".to_string(),
                "External log viewer not supported on this platform".to_string(),
            ]
        }
    }

    /// Create a subscriber for external log viewing
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }
}

impl Default for LogStreamer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global log streamer instance
static LOG_STREAMER: std::sync::OnceLock<LogStreamer> = std::sync::OnceLock::new();

/// Get the global log streamer
pub fn global_log_streamer() -> &'static LogStreamer {
    LOG_STREAMER.get_or_init(LogStreamer::new)
}

/// Initialize external log streaming
pub async fn init_external_logging() -> Result<()> {
    let streamer = global_log_streamer();
    streamer.spawn_external_viewer().await
}

/// Log a message to external viewers
pub fn log_external(message: &str) {
    global_log_streamer().log(message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_streamer_creation() {
        let streamer = LogStreamer::new();

        // Test logging
        streamer.log("Test message");

        // Test subscription
        let mut receiver = streamer.subscribe();
        streamer.log("Another test");

        // We can't easily test the actual reception in a unit test
        // since it's async, but we can verify the structure works
        assert!(receiver.try_recv().is_ok());
    }

    #[test]
    fn test_terminal_command() {
        let streamer = LogStreamer::new();
        let command = streamer.get_terminal_command();
        assert!(!command.is_empty());
        assert!(!command[0].is_empty());
    }
}
