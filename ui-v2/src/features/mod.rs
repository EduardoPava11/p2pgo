//! Feature modules for different application views

pub mod game_view;
pub mod lobby_view;
pub mod training_view;

pub use game_view::{GameView, GameAction};
pub use lobby_view::{LobbyView, LobbyAction};
pub use training_view::{TrainingView, TrainingAction};