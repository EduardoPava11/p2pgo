//! Application shell and routing

pub mod router;
pub mod app;

pub use router::{Router, View};
pub use app::P2PGoApp;