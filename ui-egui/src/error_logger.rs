use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use serde::{Serialize, Deserialize};

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Error log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: ErrorLevel,
    pub component: String,
    pub message: String,
    pub context: Option<String>,
    pub stack_trace: Option<String>,
}

/// Error logger with persistent storage
pub struct ErrorLogger {
    /// In-memory log buffer
    entries: VecDeque<LogEntry>,
    /// Maximum entries to keep in memory
    max_entries: usize,
    /// Log file path
    log_file: PathBuf,
    /// Whether to write to file
    file_logging_enabled: bool,
    /// Minimum level to log
    min_level: ErrorLevel,
}

impl ErrorLogger {
    pub fn new() -> Self {
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("p2pgo")
            .join("logs");
        
        // Create log directory if needed
        let _ = std::fs::create_dir_all(&log_dir);
        
        let log_file = log_dir.join(format!("p2pgo_{}.log", 
            Local::now().format("%Y%m%d_%H%M%S")));
        
        Self {
            entries: VecDeque::with_capacity(1000),
            max_entries: 1000,
            log_file,
            file_logging_enabled: true,
            min_level: ErrorLevel::Info,
        }
    }
    
    /// Log an entry
    pub fn log(&mut self, level: ErrorLevel, component: &str, message: &str) {
        self.log_with_context(level, component, message, None, None);
    }
    
    /// Log with additional context
    pub fn log_with_context(
        &mut self,
        level: ErrorLevel,
        component: &str,
        message: &str,
        context: Option<String>,
        stack_trace: Option<String>,
    ) {
        if (level as u8) < (self.min_level as u8) {
            return;
        }
        
        let entry = LogEntry {
            timestamp: Local::now(),
            level,
            component: component.to_string(),
            message: message.to_string(),
            context,
            stack_trace,
        };
        
        // Add to memory buffer
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry.clone());
        
        // Write to file
        if self.file_logging_enabled {
            self.write_to_file(&entry);
        }
        
        // Also print to console in debug mode
        #[cfg(debug_assertions)]
        {
            eprintln!("[{}] {} [{}]: {}", 
                entry.timestamp.format("%H:%M:%S"),
                self.level_icon(level),
                component,
                message
            );
            if let Some(ctx) = context.as_ref() {
                eprintln!("  Context: {}", ctx);
            }
        }
    }
    
    /// Log an error with automatic stack trace capture
    pub fn log_error(&mut self, component: &str, error: &anyhow::Error) {
        let stack_trace = format!("{:?}", error);
        self.log_with_context(
            ErrorLevel::Error,
            component,
            &error.to_string(),
            None,
            Some(stack_trace),
        );
    }
    
    /// Get recent entries
    pub fn get_entries(&self, max_count: usize) -> Vec<&LogEntry> {
        self.entries.iter()
            .rev()
            .take(max_count)
            .collect()
    }
    
    /// Get entries filtered by level
    pub fn get_entries_by_level(&self, level: ErrorLevel, max_count: usize) -> Vec<&LogEntry> {
        self.entries.iter()
            .rev()
            .filter(|e| e.level == level)
            .take(max_count)
            .collect()
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    
    /// Export logs to file
    pub fn export_logs(&self, path: &PathBuf) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        
        for entry in &self.entries {
            writeln!(file, "{}", self.format_entry(entry))?;
        }
        
        Ok(())
    }
    
    /// Set minimum logging level
    pub fn set_min_level(&mut self, level: ErrorLevel) {
        self.min_level = level;
    }
    
    /// Toggle file logging
    pub fn set_file_logging(&mut self, enabled: bool) {
        self.file_logging_enabled = enabled;
    }
    
    fn write_to_file(&self, entry: &LogEntry) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
        {
            let _ = writeln!(file, "{}", self.format_entry(entry));
        }
    }
    
    fn format_entry(&self, entry: &LogEntry) -> String {
        let mut output = format!(
            "[{}] {} [{}] {}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.level_string(entry.level),
            entry.component,
            entry.message
        );
        
        if let Some(ctx) = &entry.context {
            output.push_str(&format!("\n    Context: {}", ctx));
        }
        
        if let Some(trace) = &entry.stack_trace {
            output.push_str(&format!("\n    Stack trace:\n{}", 
                trace.lines()
                    .map(|line| format!("      {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        
        output
    }
    
    fn level_string(&self, level: ErrorLevel) -> &'static str {
        match level {
            ErrorLevel::Debug => "DEBUG",
            ErrorLevel::Info => "INFO",
            ErrorLevel::Warning => "WARN",
            ErrorLevel::Error => "ERROR",
            ErrorLevel::Critical => "CRITICAL",
        }
    }
    
    fn level_icon(&self, level: ErrorLevel) -> &'static str {
        match level {
            ErrorLevel::Debug => "🔍",
            ErrorLevel::Info => "ℹ️",
            ErrorLevel::Warning => "⚠️",
            ErrorLevel::Error => "❌",
            ErrorLevel::Critical => "🚨",
        }
    }
}

/// UI component for displaying logs
pub struct ErrorLogViewer {
    /// Reference to logger
    filter_level: Option<ErrorLevel>,
    /// Search query
    search_query: String,
    /// Show stack traces
    show_stack_traces: bool,
}

impl ErrorLogViewer {
    pub fn new() -> Self {
        Self {
            filter_level: None,
            search_query: String::new(),
            show_stack_traces: false,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, logger: &ErrorLogger) {
        ui.heading("📋 Error Log");
        
        // Controls
        ui.horizontal(|ui| {
            ui.label("Filter:");
            
            let filter_text = self.filter_level
                .map(|l| logger.level_string(l))
                .unwrap_or("All");
            
            egui::ComboBox::from_label("")
                .selected_text(filter_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filter_level, None, "All");
                    ui.separator();
                    ui.selectable_value(&mut self.filter_level, Some(ErrorLevel::Debug), "Debug");
                    ui.selectable_value(&mut self.filter_level, Some(ErrorLevel::Info), "Info");
                    ui.selectable_value(&mut self.filter_level, Some(ErrorLevel::Warning), "Warning");
                    ui.selectable_value(&mut self.filter_level, Some(ErrorLevel::Error), "Error");
                    ui.selectable_value(&mut self.filter_level, Some(ErrorLevel::Critical), "Critical");
                });
            
            ui.separator();
            
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);
            
            ui.separator();
            
            ui.checkbox(&mut self.show_stack_traces, "Show traces");
            
            if ui.button("Clear").clicked() {
                // Note: In real implementation, this would need mutable access to logger
            }
            
            if ui.button("Export...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("p2pgo_logs.txt")
                    .save_file()
                {
                    let _ = logger.export_logs(&path);
                }
            }
        });
        
        ui.separator();
        
        // Get filtered entries
        let entries: Vec<&LogEntry> = if let Some(level) = self.filter_level {
            logger.get_entries_by_level(level, 100)
        } else {
            logger.get_entries(100)
        };
        
        // Apply search filter
        let filtered_entries: Vec<&LogEntry> = if self.search_query.is_empty() {
            entries
        } else {
            entries.into_iter()
                .filter(|e| {
                    e.message.to_lowercase().contains(&self.search_query.to_lowercase()) ||
                    e.component.to_lowercase().contains(&self.search_query.to_lowercase())
                })
                .collect()
        };
        
        // Display entries
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for entry in filtered_entries {
                    self.render_entry(ui, entry, logger);
                }
            });
    }
    
    fn render_entry(&self, ui: &mut egui::Ui, entry: &LogEntry, logger: &ErrorLogger) {
        let color = match entry.level {
            ErrorLevel::Debug => egui::Color32::from_rgb(128, 128, 128),
            ErrorLevel::Info => egui::Color32::from_rgb(100, 150, 200),
            ErrorLevel::Warning => egui::Color32::from_rgb(255, 200, 0),
            ErrorLevel::Error => egui::Color32::from_rgb(255, 100, 100),
            ErrorLevel::Critical => egui::Color32::from_rgb(255, 0, 0),
        };
        
        ui.horizontal(|ui| {
            // Timestamp
            ui.colored_label(
                egui::Color32::from_gray(150),
                entry.timestamp.format("%H:%M:%S").to_string()
            );
            
            // Level icon
            ui.colored_label(color, logger.level_icon(entry.level));
            
            // Component
            ui.colored_label(
                egui::Color32::from_gray(200),
                format!("[{}]", entry.component)
            );
            
            // Message
            ui.colored_label(color, &entry.message);
        });
        
        // Context and stack trace
        if let Some(ctx) = &entry.context {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.colored_label(egui::Color32::from_gray(180), format!("Context: {}", ctx));
            });
        }
        
        if self.show_stack_traces {
            if let Some(trace) = &entry.stack_trace {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.collapsing("Stack trace", |ui| {
                        ui.monospace(trace);
                    });
                });
            }
        }
        
        ui.add_space(4.0);
    }
}

/// Global error logger instance
static mut ERROR_LOGGER: Option<ErrorLogger> = None;
static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

/// Get global error logger
pub fn get_error_logger() -> &'static mut ErrorLogger {
    unsafe {
        LOGGER_INIT.call_once(|| {
            ERROR_LOGGER = Some(ErrorLogger::new());
        });
        ERROR_LOGGER.as_mut().unwrap()
    }
}

/// Convenience logging macros
#[macro_export]
macro_rules! log_debug {
    ($component:expr, $($arg:tt)*) => {
        $crate::error_logger::get_error_logger().log(
            $crate::error_logger::ErrorLevel::Debug,
            $component,
            &format!($($arg)*)
        );
    };
}

#[macro_export]
macro_rules! log_info {
    ($component:expr, $($arg:tt)*) => {
        $crate::error_logger::get_error_logger().log(
            $crate::error_logger::ErrorLevel::Info,
            $component,
            &format!($($arg)*)
        );
    };
}

#[macro_export]
macro_rules! log_warn {
    ($component:expr, $($arg:tt)*) => {
        $crate::error_logger::get_error_logger().log(
            $crate::error_logger::ErrorLevel::Warning,
            $component,
            &format!($($arg)*)
        );
    };
}

#[macro_export]
macro_rules! log_error {
    ($component:expr, $($arg:tt)*) => {
        $crate::error_logger::get_error_logger().log(
            $crate::error_logger::ErrorLevel::Error,
            $component,
            &format!($($arg)*)
        );
    };
}

#[macro_export]
macro_rules! log_critical {
    ($component:expr, $($arg:tt)*) => {
        $crate::error_logger::get_error_logger().log(
            $crate::error_logger::ErrorLevel::Critical,
            $component,
            &format!($($arg)*)
        );
    };
}