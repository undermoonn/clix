#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLanguage {
    English,
    SimplifiedChinese,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppLanguageSetting {
    #[default]
    Auto,
    English,
    SimplifiedChinese,
}

impl AppLanguageSetting {
    pub fn resolve(self) -> AppLanguage {
        match self {
            Self::Auto => AppLanguage::detect_system(),
            Self::English => AppLanguage::English,
            Self::SimplifiedChinese => AppLanguage::SimplifiedChinese,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::English,
            Self::English => Self::SimplifiedChinese,
            Self::SimplifiedChinese => Self::Auto,
        }
    }

    pub fn as_config_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::English => "english",
            Self::SimplifiedChinese => "simplified_chinese",
        }
    }

    pub fn from_config_value(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("auto") {
            Some(Self::Auto)
        } else if value.eq_ignore_ascii_case("english") || value.eq_ignore_ascii_case("en") {
            Some(Self::English)
        } else if value.eq_ignore_ascii_case("simplified_chinese")
            || value.eq_ignore_ascii_case("simplified-chinese")
            || value.eq_ignore_ascii_case("zh-cn")
            || value.eq_ignore_ascii_case("schinese")
        {
            Some(Self::SimplifiedChinese)
        } else {
            None
        }
    }

    pub fn display_text(self, language: AppLanguage) -> &'static str {
        match self {
            Self::Auto => language.auto_text(),
            Self::English => language.english_text(),
            Self::SimplifiedChinese => language.simplified_chinese_text(),
        }
    }
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

    pub fn apps_text(self) -> &'static str {
        match self {
            Self::English => "Apps",
            Self::SimplifiedChinese => "应用",
        }
    }

    pub fn system_text(self) -> &'static str {
        match self {
            Self::English => "System",
            Self::SimplifiedChinese => "系统",
        }
    }

    pub fn screen_text(self) -> &'static str {
        match self {
            Self::English => "Screen",
            Self::SimplifiedChinese => "屏幕",
        }
    }

    pub fn sleep_text(self) -> &'static str {
        match self {
            Self::English => "Sleep",
            Self::SimplifiedChinese => "睡眠",
        }
    }

    pub fn reboot_text(self) -> &'static str {
        match self {
            Self::English => "Restart",
            Self::SimplifiedChinese => "重启",
        }
    }

    pub fn shutdown_text(self) -> &'static str {
        match self {
            Self::English => "Shut Down",
            Self::SimplifiedChinese => "关机",
        }
    }

    pub fn current_display_mode_text(self) -> &'static str {
        match self {
            Self::English => "Current",
            Self::SimplifiedChinese => "当前",
        }
    }

    pub fn launch_on_startup_text(self) -> &'static str {
        match self {
            Self::English => "Launch on Startup",
            Self::SimplifiedChinese => "开机自启",
        }
    }

    pub fn background_home_wake_prefix_text(self) -> &'static str {
        match self {
            Self::English => "Wake App via",
            Self::SimplifiedChinese => "通过",
        }
    }

    pub fn background_home_wake_suffix_text(self) -> &'static str {
        match self {
            Self::English => "buttons",
            Self::SimplifiedChinese => "键唤醒应用",
        }
    }

    pub fn controller_vibration_feedback_text(self) -> &'static str {
        match self {
            Self::English => "Controller Vibration Feedback",
            Self::SimplifiedChinese => "应用内手柄震动反馈",
        }
    }

    pub fn language_setting_text(self) -> &'static str {
        match self {
            Self::English => "Language",
            Self::SimplifiedChinese => "语言",
        }
    }

    pub fn client_games_detection_text(self) -> &'static str {
        match self {
            Self::English => "Games Detection",
            Self::SimplifiedChinese => "游戏识别",
        }
    }

    pub fn settings_text(self) -> &'static str {
        match self {
            Self::English => "Settings",
            Self::SimplifiedChinese => "设置",
        }
    }

    pub fn resolution_settings_text(self) -> &'static str {
        match self {
            Self::English => "Resolution Settings",
            Self::SimplifiedChinese => "分辨率设置",
        }
    }

    pub fn resolution_text(self) -> &'static str {
        match self {
            Self::English => "Resolution",
            Self::SimplifiedChinese => "分辨率",
        }
    }

    pub fn refresh_rate_text(self) -> &'static str {
        match self {
            Self::English => "Frame Rate",
            Self::SimplifiedChinese => "帧率",
        }
    }

    pub fn dlss_swapper_text(self) -> &'static str {
        match self {
            Self::English => "DLSS Swapper",
            Self::SimplifiedChinese => "DLSS Swapper",
        }
    }

    pub fn nvidia_app_text(self) -> &'static str {
        match self {
            Self::English => "NVIDIA App",
            Self::SimplifiedChinese => "NVIDIA App",
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

    pub fn auto_text(self) -> &'static str {
        match self {
            Self::English => "Auto",
            Self::SimplifiedChinese => "自动",
        }
    }

    pub fn english_text(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "英文",
        }
    }

    pub fn simplified_chinese_text(self) -> &'static str {
        match self {
            Self::English => "Simplified Chinese",
            Self::SimplifiedChinese => "简体中文",
        }
    }

    pub fn confirm_text(self) -> &'static str {
        match self {
            Self::English => "Confirm",
            Self::SimplifiedChinese => "确认",
        }
    }

    pub fn installed_app_options_text(self) -> &'static str {
        match self {
            Self::English => "Installed Apps",
            Self::SimplifiedChinese => "已安装的应用",
        }
    }

    pub fn hold_close_game_text(self) -> &'static str {
        match self {
            Self::English => "Force Exit",
            Self::SimplifiedChinese => "强制退出",
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

    pub fn steam_start_action_text(self) -> &'static str {
        match self {
            Self::English => "Start Steam",
            Self::SimplifiedChinese => "启动 Steam",
        }
    }

    pub fn steam_starting_text(self) -> &'static str {
        match self {
            Self::English => "Steam starting",
            Self::SimplifiedChinese => "Steam 启动中",
        }
    }

    pub fn steam_started_text(self) -> &'static str {
        match self {
            Self::English => "Steam started",
            Self::SimplifiedChinese => "Steam 启动完成",
        }
    }

    pub fn achievement_hidden_text(self) -> &'static str {
        match self {
            Self::English => "Show hidden achievement",
            Self::SimplifiedChinese => "显示隐藏成就",
        }
    }

    pub fn format_achievement_unlock_rate(self, percent: f32) -> String {
        match self {
            Self::English => format!("Unlocked by {:.1}% of players", percent),
            Self::SimplifiedChinese => format!("{:.1}%的玩家已解锁", percent),
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

    pub fn format_installed_size(self, size_bytes: u64) -> String {
        const KIB: f64 = 1024.0;
        const MIB: f64 = KIB * 1024.0;
        const GIB: f64 = MIB * 1024.0;

        let format_value = |value: f64| {
            let value = format!("{:.1}", value);
            value.trim_end_matches(".0").to_owned()
        };

        let bytes = size_bytes as f64;
        if bytes >= GIB {
            format!("{} GB", format_value(bytes / GIB))
        } else if bytes >= MIB {
            format!("{} MB", format_value(bytes / MIB))
        } else if bytes >= KIB {
            format!("{} KB", format_value(bytes / KIB))
        } else {
            format!("{} B", size_bytes)
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