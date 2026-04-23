#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "windows")]
type PowerBoolean = u8;

#[cfg(target_os = "windows")]
#[link(name = "PowrProf")]
extern "system" {
    fn SetSuspendState(
        hibernate: PowerBoolean,
        force_critical: PowerBoolean,
        disable_wake_event: PowerBoolean,
    ) -> PowerBoolean;
}

pub fn supported() -> bool {
    cfg!(target_os = "windows")
}

#[cfg(target_os = "windows")]
pub fn sleep_system() -> bool {
    unsafe { SetSuspendState(0, 0, 0) != 0 }
}

#[cfg(not(target_os = "windows"))]
pub fn sleep_system() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn shutdown_system() -> bool {
    Command::new("shutdown.exe")
        .args(["/s", "/t", "0"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn shutdown_system() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn reboot_system() -> bool {
    Command::new("shutdown.exe")
        .args(["/r", "/t", "0"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn reboot_system() -> bool {
    false
}