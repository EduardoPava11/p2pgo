//! Application view routing

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    Lobby,
    Game(String), // Game code
    Training,
    Settings,
}

pub struct Router {
    current_view: View,
    history: Vec<View>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            current_view: View::Lobby,
            history: Vec::new(),
        }
    }

    pub fn current(&self) -> &View {
        &self.current_view
    }

    pub fn navigate_to(&mut self, view: View) {
        if self.current_view != view {
            self.history.push(self.current_view.clone());
            self.current_view = view;
        }
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.history.pop() {
            self.current_view = prev;
        }
    }

    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
