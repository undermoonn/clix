use crate::system::external_apps::ExternalAppKind;

const HOME_MENU_COLUMNS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeMenuMove {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeMenuOption {
    ExternalApp(ExternalAppKind),
    MinimizeApp,
    CloseApp,
    Sleep,
    Shutdown,
    HalfMaxRefresh,
    MaxRefresh,
    LaunchOnStartup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HomeMenuEntry {
    pub option: HomeMenuOption,
    pub row: usize,
    pub column: usize,
    pub wide: bool,
}

#[derive(Clone, Debug, Default)]
pub struct HomeMenuLayout {
    entries: Vec<HomeMenuEntry>,
}

impl HomeMenuLayout {
    pub fn new(
        external_apps: &[ExternalAppKind],
        show_power_options: bool,
        show_launch_on_startup: bool,
    ) -> Self {
        let mut entries = Vec::new();
        let mut row = 0;

        entries.push(HomeMenuEntry {
            option: HomeMenuOption::MinimizeApp,
            row,
            column: 0,
            wide: false,
        });
        entries.push(HomeMenuEntry {
            option: HomeMenuOption::CloseApp,
            row,
            column: 1,
            wide: false,
        });
        row += 1;

        if show_power_options {
            entries.push(HomeMenuEntry {
                option: HomeMenuOption::Sleep,
                row,
                column: 0,
                wide: false,
            });
            entries.push(HomeMenuEntry {
                option: HomeMenuOption::Shutdown,
                row,
                column: 1,
                wide: false,
            });
            row += 1;
        }

        entries.push(HomeMenuEntry {
            option: HomeMenuOption::HalfMaxRefresh,
            row,
            column: 0,
            wide: false,
        });
        entries.push(HomeMenuEntry {
            option: HomeMenuOption::MaxRefresh,
            row,
            column: 1,
            wide: false,
        });
        row += 1;

        if !external_apps.is_empty() {
            for (column, kind) in external_apps.iter().copied().take(HOME_MENU_COLUMNS).enumerate() {
                entries.push(HomeMenuEntry {
                    option: HomeMenuOption::ExternalApp(kind),
                    row,
                    column,
                    wide: false,
                });
            }
            row += 1;
        }

        if show_launch_on_startup {
            entries.push(HomeMenuEntry {
                option: HomeMenuOption::LaunchOnStartup,
                row: row.saturating_sub(1),
                column: HOME_MENU_COLUMNS,
                wide: false,
            });
        }

        Self { entries }
    }

    pub fn entries(&self) -> &[HomeMenuEntry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clamp_selected(&self, selected: usize) -> usize {
        selected.min(self.entries.len().saturating_sub(1))
    }

    pub fn default_selected(&self) -> usize {
        self.entries
            .iter()
            .position(|entry| matches!(entry.option, HomeMenuOption::MinimizeApp))
            .or_else(|| {
                self.entries
                    .iter()
                    .position(|entry| matches!(entry.option, HomeMenuOption::ExternalApp(_)))
            })
            .unwrap_or(0)
    }

    pub fn option_at(&self, index: usize) -> Option<HomeMenuOption> {
        self.entries.get(index).map(|entry| entry.option)
    }

    pub fn is_shutdown_selected(&self, index: usize) -> bool {
        matches!(self.option_at(index), Some(HomeMenuOption::Shutdown))
    }

    pub fn move_selection(&self, selected: usize, direction: HomeMenuMove) -> usize {
        let selected = self.clamp_selected(selected);
        let Some(current) = self.entries.get(selected).copied() else {
            return 0;
        };

        let next = match direction {
            HomeMenuMove::Left => current
                .column
                .checked_sub(1)
                .and_then(|column| self.find_in_row(current.row, column)),
            HomeMenuMove::Right => {
                if current.wide {
                    None
                } else {
                    self.find_in_row(current.row, current.column + 1).or_else(|| {
                        (current.column == HOME_MENU_COLUMNS - 1
                            && !matches!(current.option, HomeMenuOption::LaunchOnStartup))
                            .then(|| self.find_option(HomeMenuOption::LaunchOnStartup))
                            .flatten()
                    })
                }
            }
            HomeMenuMove::Up => self.find_vertical_neighbor(current, false),
            HomeMenuMove::Down => self.find_vertical_neighbor(current, true),
        };

        next.unwrap_or(selected)
    }

    fn find_in_row(&self, row: usize, column: usize) -> Option<usize> {
        self.entries.iter().position(|entry| {
            entry.row == row
                && (entry.wide && column < HOME_MENU_COLUMNS || entry.column == column)
        })
    }

    fn find_option(&self, option: HomeMenuOption) -> Option<usize> {
        self.entries.iter().position(|entry| entry.option == option)
    }

    fn find_vertical_neighbor(&self, current: HomeMenuEntry, down: bool) -> Option<usize> {
        let max_row = self.entries.iter().map(|entry| entry.row).max()? as isize;
        let mut row = current.row as isize + if down { 1 } else { -1 };

        while (0..=max_row).contains(&row) {
            if let Some(index) = self.entries.iter().position(|entry| {
                entry.row == row as usize
                    && (entry.wide || entry.column == current.column)
            }) {
                return Some(index);
            }

            if current.column >= HOME_MENU_COLUMNS {
                if let Some(index) = self
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| entry.row == row as usize)
                    .max_by_key(|(_, entry)| entry.column)
                    .map(|(index, _)| index)
                {
                    return Some(index);
                }
            }

            row += if down { 1 } else { -1 };
        }

        None
    }
}