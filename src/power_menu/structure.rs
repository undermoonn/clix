#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerMenuOption {
    Sleep,
    Reboot,
    Shutdown,
}

const POWER_MENU_OPTIONS: [PowerMenuOption; 3] = [
    PowerMenuOption::Sleep,
    PowerMenuOption::Shutdown,
    PowerMenuOption::Reboot,
];

#[derive(Clone, Debug, Default)]
pub struct PowerMenuLayout {
    option_count: usize,
}

impl PowerMenuLayout {
    pub fn new(show_power_options: bool) -> Self {
        Self {
            option_count: if show_power_options {
                POWER_MENU_OPTIONS.len()
            } else {
                0
            },
        }
    }

    pub fn options(&self) -> &[PowerMenuOption] {
        &POWER_MENU_OPTIONS[..self.option_count]
    }

    pub fn is_empty(&self) -> bool {
        self.option_count == 0
    }

    pub fn clamp_selected(&self, selected: usize) -> usize {
        selected.min(self.option_count.saturating_sub(1))
    }

    pub fn default_selected(&self) -> usize {
        0
    }

    pub fn option_at(&self, index: usize) -> Option<PowerMenuOption> {
        self.options().get(index).copied()
    }

    pub fn move_up(&self, selected: usize) -> usize {
        self.clamp_selected(selected).saturating_sub(1)
    }

    pub fn move_down(&self, selected: usize) -> usize {
        (self.clamp_selected(selected) + 1).min(self.option_count.saturating_sub(1))
    }
}