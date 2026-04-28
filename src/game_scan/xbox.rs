use crate::game::Game;

#[cfg(target_os = "windows")]
pub fn scan_games() -> Vec<Game> {
    imp::scan_games()
}

#[cfg(not(target_os = "windows"))]
pub fn scan_games() -> Vec<Game> {
    Vec::new()
}

#[cfg(target_os = "windows")]
mod imp {
    use std::collections::HashSet;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};

    use regex::Regex;
    use serde::Deserialize;

    use crate::game::{Game, GameSource};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn scan_games() -> Vec<Game> {
        let gamingroot_started_at = Instant::now();
        let package_names = collect_gamingroot_package_names();
        log_xbox_scan_phase(
            "GamingRoot scan",
            gamingroot_started_at.elapsed(),
            &format!("{} package names", package_names.len()),
        );

        let appx_query_started_at = Instant::now();
        let Some(appx_data) = load_appx_data() else {
            log_xbox_scan_phase(
                "Appx query",
                appx_query_started_at.elapsed(),
                "query failed",
            );
            return Vec::new();
        };

        let AppxData { packages } = appx_data;
        log_xbox_scan_phase(
            "Appx query",
            appx_query_started_at.elapsed(),
            &format!("{} packages", packages.len()),
        );

        if !packages
            .iter()
            .any(|package| package.name == "Microsoft.GamingApp")
        {
            return Vec::new();
        }

        let manifest_match_started_at = Instant::now();
        let mut seen_family_names = HashSet::new();
        let mut games = Vec::new();
        let mut manifest_candidate_count = 0usize;

        for package in packages {
            if package.is_framework || package.is_resource_package {
                continue;
            }
            if !seen_family_names.insert(package.family_name.clone()) {
                continue;
            }

            let install_dir = PathBuf::from(&package.install_location);
            if !install_dir.is_dir() {
                continue;
            }

            let manifest_path = install_dir.join("AppxManifest.xml");
            let Ok(manifest_contents) = std::fs::read_to_string(manifest_path) else {
                continue;
            };

            let Some(application) = load_application_entry(&manifest_contents, &install_dir) else {
                continue;
            };

            let has_gamingroot_entry = package_names.contains(&package.name);
            if !has_gamingroot_entry && !manifest_has_xbox_game_hint(&manifest_contents) {
                continue;
            }

            manifest_candidate_count += 1;

            let launch_target = application
                .executable
                .as_deref()
                .map(|relative_path| install_dir.join(relative_path))
                .filter(|path| path.is_file());
            games.push(Game {
                source: GameSource::Xbox,
                name: application.display_name,
                install_path: install_dir,
                launch_target,
                steam_app_id: None,
                appx_id: Some(application.id),
                epic_app_name: None,
                xbox_package_family_name: Some(package.family_name),
                last_played: 0,
                playtime_minutes: 0,
                installed_size_bytes: None,
                dlss_version: None,
            });
        }

        log_xbox_scan_phase(
            "Manifest match",
            manifest_match_started_at.elapsed(),
            &format!(
                "{} candidate packages, {} games",
                manifest_candidate_count,
                games.len()
            ),
        );

        games
    }

    fn log_xbox_scan_phase(phase: &str, elapsed: Duration, details: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!(
            "[{}] xbox {} took {} ms ({})\n",
            timestamp,
            phase,
            elapsed.as_millis(),
            details
        );

        eprint!("{}", line);

        let log_path = crate::assets::cache::logs_dir().join("scan_timings.log");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = file.write_all(line.as_bytes());
        }
    }

    fn collect_gamingroot_package_names() -> HashSet<String> {
        let mut package_names = HashSet::new();

        for drive in available_drives() {
            let gaming_root_path = drive.join(".GamingRoot");
            let Some(target_dir) = read_gamingroot_target_dir(&drive, &gaming_root_path) else {
                continue;
            };

            let Ok(entries) = std::fs::read_dir(target_dir) else {
                continue;
            };

            for entry in entries.filter_map(Result::ok) {
                let config_path = entry.path().join("Content").join("MicrosoftGame.config");
                let Some(package_name) = load_config_identity_name(&config_path) else {
                    continue;
                };

                package_names.insert(package_name);
            }
        }

        package_names
    }

    fn available_drives() -> Vec<PathBuf> {
        (b'A'..=b'Z')
            .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
            .filter(|path| path.exists())
            .collect()
    }

    fn read_gamingroot_target_dir(drive_root: &Path, gaming_root_path: &Path) -> Option<PathBuf> {
        let bytes = std::fs::read(gaming_root_path).ok()?;
        if bytes.len() <= 5 || &bytes[..4] != b"RGBX" {
            return None;
        }

        let suffix = bytes[5..]
            .iter()
            .copied()
            .filter(|byte| *byte != 0)
            .map(char::from)
            .collect::<String>();
        let target_dir = PathBuf::from(format!("{}{}", drive_root.display(), suffix));
        target_dir.is_dir().then_some(target_dir)
    }

    fn load_config_identity_name(config_path: &Path) -> Option<String> {
        let contents = std::fs::read_to_string(config_path).ok()?;
        let config_version = config_version_regex()
            .captures(&contents)
            .and_then(|captures| captures.get(1).map(|value| value.as_str()))?;
        if config_version != "0" && config_version != "1" {
            return None;
        }

        identity_name_regex()
            .captures(&contents)
            .and_then(|captures| {
                captures
                    .get(1)
                    .map(|value| value.as_str().trim().to_owned())
            })
            .filter(|value| !value.is_empty())
    }

    fn load_appx_data() -> Option<AppxData> {
        let script = r#"$packages = Get-AppxPackage | Select-Object Name, PackageFamilyName, InstallLocation, IsFramework, IsResourcePackage;
[PSCustomObject]@{ Packages = @($packages) }"#;

        let query = run_powershell_json::<AppxQueryResult>(script)?;

        Some(AppxData {
            packages: query.packages,
        })
    }

    fn load_application_entry(
        manifest_contents: &str,
        install_dir: &Path,
    ) -> Option<AppManifestApplication> {
        let mut first_application = None;
        let manifest_display_name = capture_manifest_display_name(manifest_contents);

        if manifest_display_name.is_some_and(is_manifest_resource_reference) {
            return None;
        }

        for application_tag in application_tag_regex()
            .find_iter(manifest_contents)
            .map(|value| value.as_str())
        {
            let Some(id) = application_id_regex()
                .captures(application_tag)
                .and_then(|captures| {
                    captures
                        .get(1)
                        .map(|value| value.as_str().trim().to_owned())
                })
            else {
                continue;
            };

            let executable = application_executable_regex()
                .captures(application_tag)
                .and_then(|captures| {
                    captures
                        .get(1)
                        .map(|value| value.as_str().trim().to_owned())
                });
            let fallback_display_name = install_dir
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Xbox Game")
                .to_owned();
            let application = AppManifestApplication {
                display_name: manifest_display_name
                    .map(str::to_owned)
                    .unwrap_or(fallback_display_name),
                id,
                executable,
            };

            if first_application.is_none() {
                first_application = Some(application);
            }
        }

        first_application
    }

    fn manifest_has_xbox_game_hint(manifest_contents: &str) -> bool {
        manifest_contents.contains("Microsoft.Xbox.Services")
            || manifest_contents.contains("ms-xbl-")
    }

    fn capture_manifest_display_name(manifest_contents: &str) -> Option<&str> {
        manifest_display_name_element_regex()
            .captures(manifest_contents)
            .and_then(|captures| captures.get(1).map(|value| value.as_str().trim()))
            .filter(|value| !value.is_empty())
            .or_else(|| {
                manifest_display_name_attribute_regex()
                    .captures(manifest_contents)
                    .and_then(|captures| captures.get(1).map(|value| value.as_str().trim()))
                    .filter(|value| !value.is_empty())
            })
    }

    fn is_manifest_resource_reference(value: &str) -> bool {
        value
            .get(.."ms-resource:".len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("ms-resource:"))
    }

    fn run_powershell_json<T>(script: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let command = format!(
            "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::UTF8; $ProgressPreference = 'SilentlyContinue'; {} | ConvertTo-Json -Compress",
            script
        );
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &command])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8(output.stdout).ok()?;
        let json = text.trim();
        if json.is_empty() {
            return None;
        }

        serde_json::from_str(json).ok()
    }

    fn config_version_regex() -> &'static Regex {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        REGEX.get_or_init(|| Regex::new("<Game\\b[^>]*\\bconfigVersion=\"([^\"]+)\"").unwrap())
    }

    fn identity_name_regex() -> &'static Regex {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        REGEX.get_or_init(|| Regex::new("<Identity\\b[^>]*\\bName=\"([^\"]+)\"").unwrap())
    }

    fn application_tag_regex() -> &'static Regex {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        REGEX.get_or_init(|| Regex::new(r#"<Application\b[^>]*>"#).unwrap())
    }

    fn application_id_regex() -> &'static Regex {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        REGEX.get_or_init(|| Regex::new("\\bId=\"([^\"]+)\"").unwrap())
    }

    fn application_executable_regex() -> &'static Regex {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        REGEX.get_or_init(|| Regex::new("\\bExecutable=\"([^\"]+)\"").unwrap())
    }

    fn manifest_display_name_element_regex() -> &'static Regex {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        REGEX.get_or_init(|| Regex::new("<DisplayName>([^<]+)</DisplayName>").unwrap())
    }

    fn manifest_display_name_attribute_regex() -> &'static Regex {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        REGEX.get_or_init(|| Regex::new("\\bDisplayName=\"([^\"]+)\"").unwrap())
    }

    #[derive(Deserialize)]
    struct AppxPackageRecord {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "PackageFamilyName")]
        family_name: String,
        #[serde(rename = "InstallLocation")]
        install_location: String,
        #[serde(rename = "IsFramework")]
        is_framework: bool,
        #[serde(rename = "IsResourcePackage")]
        is_resource_package: bool,
    }

    #[derive(Deserialize)]
    struct AppxQueryResult {
        #[serde(rename = "Packages")]
        packages: Vec<AppxPackageRecord>,
    }

    struct AppxData {
        packages: Vec<AppxPackageRecord>,
    }

    struct AppManifestApplication {
        display_name: String,
        id: String,
        executable: Option<String>,
    }
}
