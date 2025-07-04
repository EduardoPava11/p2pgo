//! Structured logging with correlation IDs for distributed tracing

use serde::{Serialize, Deserialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use uuid::Uuid;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Correlation ID for tracking requests across the distributed system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CorrelationId {
    /// Unique request ID
    pub request_id: String,
    /// Session ID for the user
    pub session_id: String,
    /// Game ID if in game context
    pub game_id: Option<String>,
    /// Peer ID for P2P tracking
    pub peer_id: Option<String>,
}

impl CorrelationId {
    /// Create a new correlation ID for a request
    pub fn new(session_id: String) -> Self {
        let request_id = format!(
            "{}-{}",
            Uuid::new_v4().simple(),
            REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        
        Self {
            request_id,
            session_id,
            game_id: None,
            peer_id: None,
        }
    }
    
    /// Add game context
    pub fn with_game(mut self, game_id: String) -> Self {
        self.game_id = Some(game_id);
        self
    }
    
    /// Add peer context
    pub fn with_peer(mut self, peer_id: String) -> Self {
        self.peer_id = Some(peer_id);
        self
    }
}

/// Structured log entry
#[derive(Debug, Serialize)]
pub struct LogEntry {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Log level
    pub level: String,
    /// Component name
    pub component: String,
    /// Log message
    pub message: String,
    /// Correlation ID
    pub correlation: CorrelationId,
    /// Additional context fields
    #[serde(flatten)]
    pub fields: serde_json::Value,
}

impl LogEntry {
    pub fn new(
        level: &str,
        component: &str,
        message: &str,
        correlation: CorrelationId,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: level.to_string(),
            component: component.to_string(),
            message: message.to_string(),
            correlation,
            fields: serde_json::json!({}),
        }
    }
    
    /// Add additional context fields
    pub fn with_fields(mut self, fields: serde_json::Value) -> Self {
        self.fields = fields;
        self
    }
}

/// Structured logger trait
pub trait StructuredLogger: Send + Sync {
    /// Log a structured entry
    fn log(&self, entry: LogEntry);
    
    /// Log with correlation ID
    fn log_with_correlation(
        &self,
        level: &str,
        component: &str,
        message: &str,
        correlation: CorrelationId,
        fields: Option<serde_json::Value>,
    ) {
        let mut entry = LogEntry::new(level, component, message, correlation);
        if let Some(fields) = fields {
            entry = entry.with_fields(fields);
        }
        self.log(entry);
    }
}

/// JSON logger implementation
pub struct JsonLogger;

impl StructuredLogger for JsonLogger {
    fn log(&self, entry: LogEntry) {
        // Output as single-line JSON for log aggregation systems
        if let Ok(json) = serde_json::to_string(&entry) {
            println!("{}", json);
        }
    }
}

/// Logger with contextual information
pub struct ContextLogger {
    inner: Box<dyn StructuredLogger>,
    correlation: CorrelationId,
    component: String,
}

impl ContextLogger {
    pub fn new(
        inner: Box<dyn StructuredLogger>,
        correlation: CorrelationId,
        component: &str,
    ) -> Self {
        Self {
            inner,
            correlation,
            component: component.to_string(),
        }
    }
    
    pub fn debug(&self, message: &str) {
        self.log("DEBUG", message, None);
    }
    
    pub fn info(&self, message: &str) {
        self.log("INFO", message, None);
    }
    
    pub fn warn(&self, message: &str) {
        self.log("WARN", message, None);
    }
    
    pub fn error(&self, message: &str) {
        self.log("ERROR", message, None);
    }
    
    pub fn with_fields(&self, message: &str, fields: serde_json::Value) {
        self.log("INFO", message, Some(fields));
    }
    
    fn log(&self, level: &str, message: &str, fields: Option<serde_json::Value>) {
        self.inner.log_with_correlation(
            level,
            &self.component,
            message,
            self.correlation.clone(),
            fields,
        );
    }
}

/// Performance timer for measuring operations
pub struct PerfTimer {
    start: SystemTime,
    operation: String,
    logger: ContextLogger,
}

impl PerfTimer {
    pub fn new(operation: &str, logger: ContextLogger) -> Self {
        let start = SystemTime::now();
        logger.debug(&format!("Starting {}", operation));
        
        Self {
            start,
            operation: operation.to_string(),
            logger,
        }
    }
}

impl Drop for PerfTimer {
    fn drop(&mut self) {
        if let Ok(duration) = self.start.elapsed() {
            self.logger.with_fields(
                &format!("Completed {}", self.operation),
                serde_json::json!({
                    "duration_ms": duration.as_millis(),
                    "operation": self.operation,
                })
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_correlation_id_creation() {
        let corr = CorrelationId::new("session123".to_string())
            .with_game("game456".to_string())
            .with_peer("peer789".to_string());
        
        assert_eq!(corr.session_id, "session123");
        assert_eq!(corr.game_id, Some("game456".to_string()));
        assert_eq!(corr.peer_id, Some("peer789".to_string()));
        assert!(!corr.request_id.is_empty());
    }
    
    #[test]
    fn test_log_entry_serialization() {
        let corr = CorrelationId::new("test-session".to_string());
        let entry = LogEntry::new("INFO", "test-component", "Test message", corr)
            .with_fields(serde_json::json!({"key": "value"}));
        
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"level\":\"INFO\""));
        assert!(json.contains("\"component\":\"test-component\""));
        assert!(json.contains("\"message\":\"Test message\""));
        assert!(json.contains("\"key\":\"value\""));
    }
}