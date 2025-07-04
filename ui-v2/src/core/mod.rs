//! Core UI components and theme

pub mod theme;
pub mod button;
pub mod card;
pub mod input;

// Re-export commonly used items
pub use theme::{apply_theme, Colors, Spacing, Styles, Typography};
pub use button::{StyledButton, ButtonStyle, ButtonSize, primary_button, secondary_button, danger_button, ghost_button};
pub use card::{Card, CardElevation, SectionCard};
pub use input::{StyledInput, LabeledInput};