#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMenuOption {
    Details,
    ForceClose,
    ToggleHomeVisibility,
}

#[derive(Clone, Debug, Default)]
pub struct GameMenuLayout {
    options: Vec<GameMenuOption>,
}

impl GameMenuLayout {
    pub fn new(show_force_close: bool, show_details: bool) -> Self {
        let mut options = Vec::with_capacity(3);
        if show_details {
            options.push(GameMenuOption::Details);
        }
        if show_force_close {
            options.push(GameMenuOption::ForceClose);
        }
        options.push(GameMenuOption::ToggleHomeVisibility);
        Self { options }
    }

    pub fn options(&self) -> &[GameMenuOption] {
        &self.options
    }

    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    pub fn clamp_selected(&self, selected: usize) -> usize {
        selected.min(self.options.len().saturating_sub(1))
    }

    pub fn default_selected(&self) -> usize {
        0
    }

    pub fn option_at(&self, index: usize) -> Option<GameMenuOption> {
        self.options.get(index).copied()
    }

    pub fn move_up(&self, selected: usize) -> usize {
        self.clamp_selected(selected).saturating_sub(1)
    }

    pub fn move_down(&self, selected: usize) -> usize {
        (self.clamp_selected(selected) + 1).min(self.options.len().saturating_sub(1))
    }
}
