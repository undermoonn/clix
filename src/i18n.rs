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

    pub fn close_app_text(self) -> &'static str {
        match self {
            Self::English => "Close App",
            Self::SimplifiedChinese => "关闭应用",
        }
    }

    pub fn minimize_app_text(self) -> &'static str {
        match self {
            Self::English => "Minimize",
            Self::SimplifiedChinese => "最小化",
        }
    }

    pub fn set_display_resolution_text(self) -> &'static str {
        match self {
            Self::English => "Set Display Resolution",
            Self::SimplifiedChinese => "设置显示分辨率",
        }
    }

    pub fn current_display_mode_text(self) -> &'static str {
        match self {
            Self::English => "Current",
            Self::SimplifiedChinese => "当前",
        }
    }

    pub fn startup_settings_text(self) -> &'static str {
        match self {
            Self::English => "Startup",
            Self::SimplifiedChinese => "启动",
        }
    }

    pub fn launch_on_startup_text(self) -> &'static str {
        match self {
            Self::English => "Launch on Startup",
            Self::SimplifiedChinese => "开机自启",
        }
    }

    pub fn enabled_text(self) -> &'static str {
        match self {
            Self::English => "On",
            Self::SimplifiedChinese => "开启",
        }
    }

    pub fn disabled_text(self) -> &'static str {
        match self {
            Self::English => "Off",
            Self::SimplifiedChinese => "关闭",
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

    pub fn refresh_text(self) -> &'static str {
        match self {
            Self::English => "Refresh",
            Self::SimplifiedChinese => "刷新",
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

    pub fn achievement_hidden_text(self) -> &'static str {
        match self {
            Self::English => "Show hidden achievement",
            Self::SimplifiedChinese => "显示隐藏成就",
        }
    }

    pub fn unlock_rate_high_to_low_text(self) -> &'static str {
        match self {
            Self::English => "Rate High-Low",
            Self::SimplifiedChinese => "解锁率高到低",
        }
    }

    pub fn unlock_rate_low_to_high_text(self) -> &'static str {
        match self {
            Self::English => "Rate Low-High",
            Self::SimplifiedChinese => "解锁率低到高",
        }
    }

    pub fn window_title(self) -> &'static str {
        match self {
            Self::English => "Big Screen Launcher",
            Self::SimplifiedChinese => "Big Screen Launcher",
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