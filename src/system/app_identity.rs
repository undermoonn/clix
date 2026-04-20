#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::OsStr;
    use std::iter;
    use std::os::windows::ffi::OsStrExt;

    use winapi::Interface;
    use winapi::shared::ntdef::LPCWSTR;
    use winapi::shared::windef::HWND;
    use winapi::shared::winerror::SUCCEEDED;
    use winapi::shared::wtypes::VT_LPWSTR;
    use winapi::um::propidl::PROPVARIANT;
    use winapi::um::propkey::{
        PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand,
        PKEY_AppUserModel_RelaunchDisplayNameResource, PKEY_AppUserModel_RelaunchIconResource,
    };
    use winapi::um::propsys::IPropertyStore;
    use winapi::um::shellapi::SHGetPropertyStoreForWindow;

    const APP_USER_MODEL_ID: &str = "BigScreenLauncher";

    #[link(name = "shell32")]
    extern "system" {
        fn SetCurrentProcessExplicitAppUserModelID(app_id: LPCWSTR) -> i32;
    }

    pub fn configure_root_window(hwnd: isize, display_name: &str) {
        unsafe {
            let app_id = wide(APP_USER_MODEL_ID);
            let _ = SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr());

            let Some(exe_path) = std::env::current_exe().ok() else {
                return;
            };

            let relaunch_command = format!("\"{}\"", exe_path.display());
            let relaunch_icon = format!("{},0", exe_path.display());

            let mut store: *mut IPropertyStore = std::ptr::null_mut();
            if !SUCCEEDED(SHGetPropertyStoreForWindow(
                hwnd as HWND,
                &IPropertyStore::uuidof(),
                &mut store as *mut *mut IPropertyStore as *mut *mut _,
            )) || store.is_null()
            {
                return;
            }

            let app_id = wide(APP_USER_MODEL_ID);
            let relaunch_command = wide(&relaunch_command);
            let display_name = wide(display_name);
            let relaunch_icon = wide(&relaunch_icon);

            let _ = set_string_property(store, &PKEY_AppUserModel_RelaunchCommand, &relaunch_command);
            let _ = set_string_property(
                store,
                &PKEY_AppUserModel_RelaunchDisplayNameResource,
                &display_name,
            );
            let _ = set_string_property(store, &PKEY_AppUserModel_RelaunchIconResource, &relaunch_icon);
            let _ = set_string_property(store, &PKEY_AppUserModel_ID, &app_id);
            let _ = (*store).Commit();
            let _ = (*store).Release();
        }
    }

    unsafe fn set_string_property(
        store: *mut IPropertyStore,
        key: &winapi::shared::wtypes::PROPERTYKEY,
        value: &[u16],
    ) -> i32 {
        let mut prop = PROPVARIANT {
            vt: VT_LPWSTR as u16,
            wReserved1: 0,
            wReserved2: 0,
            wReserved3: 0,
            data: std::mem::zeroed(),
        };
        *prop.data.pwszVal_mut() = value.as_ptr() as *mut _;
        (*store).SetValue(key, &prop)
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(iter::once(0))
            .collect()
    }
}

#[cfg(target_os = "windows")]
pub fn configure_root_window(hwnd: isize, display_name: &str) {
    imp::configure_root_window(hwnd, display_name);
}

#[cfg(not(target_os = "windows"))]
pub fn configure_root_window(_hwnd: isize, _display_name: &str) {}