use std::path::{Path, PathBuf};

pub fn is_enabled() -> bool {
    imp::is_enabled()
}

pub fn set_enabled(enabled: bool) -> bool {
    imp::set_enabled(enabled)
}

#[cfg(target_os = "windows")]
mod imp {
    use super::{normalize_path, parse_command_path};
    use std::io;
    use windows::core::HSTRING;
    use windows::ApplicationModel::{StartupTask, StartupTaskState};
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN_VALUE_NAME: &str = "BigScreenLauncher";
    const LEGACY_RUN_VALUE_NAMES: &[&str] = &["Clix"];
    // Must match the desktop:StartupTask TaskId in the MSIX/Appx manifest.
    const STARTUP_TASK_ID: &str = "BigScreenLauncherStartup";

    pub fn is_enabled() -> bool {
        if is_packaged_process() {
            clear_legacy_run_values();
            return packaged_startup_task_state().is_some_and(|state| state == StartupTaskState::Enabled);
        }

        clear_legacy_run_values();

        let Some(current_exe) = current_exe_path() else {
            return false;
        };
        let Some(registered_exe) = registered_exe_path() else {
            return false;
        };

        current_exe == registered_exe
    }

    pub fn set_enabled(enabled: bool) -> bool {
        if is_packaged_process() {
            clear_legacy_run_values();
            return set_packaged_enabled(enabled);
        }

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok((key, _)) = hkcu.create_subkey(RUN_KEY_PATH) else {
            return false;
        };

        clear_legacy_run_values_for_key(&key);

        if enabled {
            let Some(command) = current_startup_command() else {
                return false;
            };
            key.set_value(RUN_VALUE_NAME, &command).is_ok()
        } else {
            delete_run_value(&key, RUN_VALUE_NAME)
        }
    }

    fn current_startup_command() -> Option<String> {
        std::env::current_exe()
            .ok()
            .map(|path| format!("\"{}\"", path.display()))
    }

    fn packaged_startup_task() -> Option<StartupTask> {
        let operation = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID)).ok()?;
        operation.get().ok()
    }

    fn packaged_startup_task_state() -> Option<StartupTaskState> {
        packaged_startup_task()?.State().ok()
    }

    fn set_packaged_enabled(enabled: bool) -> bool {
        let Some(task) = packaged_startup_task() else {
            return false;
        };

        let Some(state) = task.State().ok() else {
            return false;
        };

        if enabled {
            match state {
                StartupTaskState::Enabled => true,
                StartupTaskState::Disabled => {
                    let Ok(operation) = task.RequestEnableAsync() else {
                        return false;
                    };

                    operation
                        .get()
                        .ok()
                        .is_some_and(|new_state| new_state == StartupTaskState::Enabled)
                }
                StartupTaskState::DisabledByUser | StartupTaskState::DisabledByPolicy => false,
                _ => false,
            }
        } else {
            match state {
                StartupTaskState::Enabled => task.Disable().is_ok() && task.State().ok() != Some(StartupTaskState::Enabled),
                _ => true,
            }
        }
    }

    fn is_packaged_process() -> bool {
        let Some(path) = std::env::current_exe().ok() else {
            return false;
        };

        path.ancestors().any(|ancestor| {
            ancestor
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("WindowsApps"))
        })
    }

    fn current_exe_path() -> Option<String> {
        std::env::current_exe().ok().map(normalize_path)
    }

    fn registered_exe_path() -> Option<String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey_with_flags(RUN_KEY_PATH, KEY_READ).ok()?;
        let command: String = key.get_value(RUN_VALUE_NAME).ok()?;
        parse_command_path(&command).map(normalize_path)
    }

    fn delete_run_value(key: &RegKey, value_name: &str) -> bool {
        match key.delete_value(value_name) {
            Ok(()) => true,
            Err(err) if err.kind() == io::ErrorKind::NotFound => true,
            Err(_) => false,
        }
    }

    fn clear_legacy_run_values() {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok((key, _)) = hkcu.create_subkey(RUN_KEY_PATH) else {
            return;
        };

        clear_legacy_run_values_for_key(&key);
    }

    fn clear_legacy_run_values_for_key(key: &RegKey) {
        for value_name in LEGACY_RUN_VALUE_NAMES {
            let _ = delete_run_value(key, value_name);
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    pub fn is_enabled() -> bool {
        false
    }

    pub fn set_enabled(_enabled: bool) -> bool {
        false
    }
}

fn normalize_path(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn parse_command_path(command: &str) -> Option<PathBuf> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("\\\"") {
        let end = rest.find("\\\"").or_else(|| rest.find('"'))?;
        return Some(PathBuf::from(&rest[..end]));
    }

    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"').or_else(|| rest.find("\\\""))?;
        return Some(PathBuf::from(&rest[..end]));
    }

    trimmed
        .split_whitespace()
        .next()
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::parse_command_path;
    use std::path::PathBuf;

    #[test]
    fn parse_command_path_reads_quoted_executable() {
        let path = parse_command_path(
            r#"\"C:\Program Files\Big Screen Launcher\big-screen-launcher.exe\" --background"#,
        );

        assert_eq!(
            path,
            Some(PathBuf::from(
                r"C:\Program Files\Big Screen Launcher\big-screen-launcher.exe",
            ))
        );
    }

    #[test]
    fn parse_command_path_reads_plain_quoted_executable() {
        let path = parse_command_path(
            r#""C:\Program Files\Big Screen Launcher\big-screen-launcher.exe" --background"#,
        );

        assert_eq!(
            path,
            Some(PathBuf::from(
                r"C:\Program Files\Big Screen Launcher\big-screen-launcher.exe",
            ))
        );
    }

    #[test]
    fn parse_command_path_reads_unquoted_executable() {
        let path = parse_command_path(r"C:\BigScreenLauncher\big-screen-launcher.exe --background");

        assert_eq!(
            path,
            Some(PathBuf::from(r"C:\BigScreenLauncher\big-screen-launcher.exe"))
        );
    }
}