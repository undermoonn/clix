#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLanguage {
    English,
    SimplifiedChinese,
}

impl AppLanguage {
    pub fn detect_system() -> Self {
        sys_locale::get_locale()
            .as_deref()
            .map(Self::from_system_locale)
            .unwrap_or(Self::English)
    }

    fn from_system_locale(locale: &str) -> Self {
        let normalized = locale.replace('_', "-").to_ascii_lowercase();
        if normalized.starts_with("zh-cn")
            || normalized.starts_with("zh-sg")
            || normalized == "zh-hans"
            || normalized.starts_with("zh-hans-")
            || normalized == "zh"
        {
            Self::SimplifiedChinese
        } else if normalized.starts_with("en") {
            Self::English
        } else {
            Self::English
        }
    }

    pub fn steam_language_key(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::SimplifiedChinese => "schinese",
        }
    }

    pub fn achievement_empty_text(self) -> &'static str {
        match self {
            Self::English => "No achievements",
            Self::SimplifiedChinese => "暂无成就",
        }
    }

    pub fn no_description_text(self) -> &'static str {
        match self {
            Self::English => "No description",
            Self::SimplifiedChinese => "暂无描述",
        }
    }

    pub fn back_text(self) -> &'static str {
        match self {
            Self::English => "Back",
            Self::SimplifiedChinese => "返回",
        }
    }

    pub fn hold_quit_text(self) -> &'static str {
        match self {
            Self::English => "Hold Quit",
            Self::SimplifiedChinese => "长按退出",
        }
    }

    pub fn hold_close_game_text(self) -> &'static str {
        match self {
            Self::English => "Force Exit",
            Self::SimplifiedChinese => "强制退出",
        }
    }

    pub fn scroll_text(self) -> &'static str {
        match self {
            Self::English => "Scroll",
            Self::SimplifiedChinese => "滚动",
        }
    }

    pub fn start_text(self) -> &'static str {
        match self {
            Self::English => "Launch",
            Self::SimplifiedChinese => "启动",
        }
    }

    pub fn achievements_text(self) -> &'static str {
        match self {
            Self::English => "Details",
            Self::SimplifiedChinese => "详情",
        }
    }

    pub fn window_title(self) -> &'static str {
        match self {
            Self::English => "Clix",
            Self::SimplifiedChinese => "Clix",
        }
    }

    pub fn format_playtime(self, playtime_minutes: u32) -> String {
        if playtime_minutes >= 60 {
            let hours = playtime_minutes as f32 / 60.0;
            let value = format!("{:.1}", hours);
            let value = value.trim_end_matches(".0");
            match self {
                Self::English => format!("{} hrs", value),
                Self::SimplifiedChinese => format!("{} 小时", value),
            }
        } else if playtime_minutes > 0 {
            match self {
                Self::English => format!("{} min", playtime_minutes),
                Self::SimplifiedChinese => format!("{} 分钟", playtime_minutes),
            }
        } else {
            String::new()
        }
    }

    pub fn format_achievement_progress(self, unlocked: Option<u32>, total: u32) -> String {
        match (self, unlocked) {
            (Self::English, Some(unlocked)) => format!("{}/{} achievements", unlocked, total),
            (Self::English, None) => format!("--/{} achievements", total),
            (Self::SimplifiedChinese, Some(unlocked)) => format!("{}/{} 个成就", unlocked, total),
            (Self::SimplifiedChinese, None) => format!("--/{} 个成就", total),
        }
    }
}