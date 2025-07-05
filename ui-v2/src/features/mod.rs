//! Feature modules for different application views

pub mod game_view;
pub mod lobby_view;
pub mod training_view;

pub use game_view::{GameAction, GameView};
pub use lobby_view::{LobbyAction, LobbyView};
pub use training_view::{TrainingAction, TrainingView};
