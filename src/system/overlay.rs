use crate::i18n::AppLanguage;

#[cfg(target_os = "windows")]
mod windows {
    use std::ptr;
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::{mpsc, Mutex, OnceLock};
    use std::time::Duration;

    use crate::i18n::AppLanguage;

    use winapi::shared::minwindef::{FALSE, LPARAM, LRESULT, TRUE, UINT, WPARAM};
    use winapi::shared::windef::{HBRUSH, HGDIOBJ, HWND, RECT};
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winapi::um::wingdi::{
        CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, RoundRect, SelectObject, SetBkMode,
        SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH,
        FF_SWISS, FW_BOLD, FW_NORMAL, OUT_DEFAULT_PRECIS, PS_SOLID, TRANSPARENT,
    };
    use winapi::um::winuser::{
        BeginPaint, CreateWindowExW, DefWindowProcW, DispatchMessageW, DrawTextW, EndPaint,
        FillRect, GetClientRect, GetSystemMetrics, InvalidateRect, LoadCursorW, PeekMessageW,
        PostQuitMessage, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos, ShowWindow,
        TranslateMessage, CS_HREDRAW, CS_VREDRAW, DT_CENTER, DT_SINGLELINE, DT_VCENTER,
        DT_WORDBREAK, HWND_TOPMOST, IDC_ARROW, LWA_ALPHA, MSG, PAINTSTRUCT, PM_REMOVE,
        SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, WM_DESTROY, WM_ERASEBKGND,
        WM_PAINT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
        WS_POPUP,
    };

    const WINDOW_CLASS_NAME: &str = "BigScreenLauncherSystemOverlay";
    const OVERLAY_ALPHA: u8 = 232;
    const LANG_ENGLISH: u8 = 0;
    const LANG_SIMPLIFIED_CHINESE: u8 = 1;

    static COMMAND_TX: OnceLock<Mutex<Option<mpsc::Sender<OverlayCommand>>>> = OnceLock::new();
    static LANGUAGE: AtomicU8 = AtomicU8::new(LANG_ENGLISH);

    enum OverlayCommand {
        Show,
        Hide,
    }

    struct OverlayWindow {
        hwnd: HWND,
        visible: bool,
    }

    pub(super) fn start(language: AppLanguage) {
        set_language(language);

        let slot = COMMAND_TX.get_or_init(|| Mutex::new(None));
        let mut sender = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if sender.is_some() {
            return;
        }

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || run_overlay_thread(rx));
        *sender = Some(tx);
    }

    pub(super) fn set_language(language: AppLanguage) {
        LANGUAGE.store(language_to_index(language), Ordering::Release);
    }

    pub(super) fn set_visible(visible: bool) {
        let Some(slot) = COMMAND_TX.get() else {
            return;
        };
        let sender = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(sender) = sender.as_ref() else {
            return;
        };

        let command = if visible {
            OverlayCommand::Show
        } else {
            OverlayCommand::Hide
        };
        let _ = sender.send(command);
    }

    fn language_to_index(language: AppLanguage) -> u8 {
        match language {
            AppLanguage::English => LANG_ENGLISH,
            AppLanguage::SimplifiedChinese => LANG_SIMPLIFIED_CHINESE,
        }
    }

    fn current_language() -> AppLanguage {
        match LANGUAGE.load(Ordering::Acquire) {
            LANG_SIMPLIFIED_CHINESE => AppLanguage::SimplifiedChinese,
            _ => AppLanguage::English,
        }
    }

    fn run_overlay_thread(rx: mpsc::Receiver<OverlayCommand>) {
        let Some(mut window) = (unsafe { create_overlay_window() }) else {
            return;
        };

        loop {
            unsafe {
                let mut message: MSG = std::mem::zeroed();
                while PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if message.message == winapi::um::winuser::WM_QUIT {
                        return;
                    }
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }

            match rx.recv_timeout(Duration::from_millis(16)) {
                Ok(OverlayCommand::Show) => unsafe { window.show() },
                Ok(OverlayCommand::Hide) => unsafe { window.hide() },
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    unsafe fn create_overlay_window() -> Option<OverlayWindow> {
        let instance = GetModuleHandleW(ptr::null());
        let class_name = wide(WINDOW_CLASS_NAME);
        let window_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: ptr::null_mut(),
            hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&window_class);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            wide("Big Screen Launcher Overlay").as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null_mut(),
        );

        if hwnd.is_null() {
            return None;
        }

        SetLayeredWindowAttributes(hwnd, 0, OVERLAY_ALPHA, LWA_ALPHA);
        Some(OverlayWindow {
            hwnd,
            visible: false,
        })
    }

    impl OverlayWindow {
        unsafe fn show(&mut self) {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1);
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            InvalidateRect(self.hwnd, ptr::null(), TRUE);
            self.visible = true;
        }

        unsafe fn hide(&mut self) {
            if self.visible {
                ShowWindow(self.hwnd, SW_HIDE);
                self.visible = false;
            }
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        message: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                paint_overlay(hwnd);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    unsafe fn paint_overlay(hwnd: HWND) {
        let mut paint: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut paint);
        if hdc.is_null() {
            return;
        }

        let mut rect: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rect);

        let mask_brush = CreateSolidBrush(rgb(4, 6, 10));
        FillRect(hdc, &rect, mask_brush as HBRUSH);
        DeleteObject(mask_brush as HGDIOBJ);

        draw_card(hdc, &rect, current_language());
        EndPaint(hwnd, &paint);
    }

    unsafe fn draw_card(hdc: winapi::shared::windef::HDC, rect: &RECT, language: AppLanguage) {
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let scale = ((width as f32 / 1920.0).min(height as f32 / 1080.0)).max(0.65);
        let card_width = (560.0 * scale).round() as i32;
        let card_height = (260.0 * scale).round() as i32;
        let left = rect.left + (width - card_width) / 2;
        let top = rect.top + (height - card_height) / 2;
        let right = left + card_width;
        let bottom = top + card_height;
        let radius = (28.0 * scale).round() as i32;

        let card_brush = CreateSolidBrush(rgb(22, 28, 38));
        let border_pen = CreatePen(
            PS_SOLID as i32,
            (2.0 * scale).round() as i32,
            rgb(88, 110, 145),
        );
        let old_brush = SelectObject(hdc, card_brush as HGDIOBJ);
        let old_pen = SelectObject(hdc, border_pen as HGDIOBJ);
        RoundRect(hdc, left, top, right, bottom, radius, radius);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(border_pen as HGDIOBJ);
        DeleteObject(card_brush as HGDIOBJ);

        let accent_brush = CreateSolidBrush(rgb(96, 165, 250));
        let accent_pen = CreatePen(PS_SOLID as i32, 1, rgb(96, 165, 250));
        let old_brush = SelectObject(hdc, accent_brush as HGDIOBJ);
        let old_pen = SelectObject(hdc, accent_pen as HGDIOBJ);
        let icon_size = (64.0 * scale).round() as i32;
        let icon_left = left + (card_width - icon_size) / 2;
        let icon_top = top + (34.0 * scale).round() as i32;
        RoundRect(
            hdc,
            icon_left,
            icon_top,
            icon_left + icon_size,
            icon_top + icon_size,
            icon_size / 2,
            icon_size / 2,
        );
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(accent_pen as HGDIOBJ);
        DeleteObject(accent_brush as HGDIOBJ);

        SetBkMode(hdc, TRANSPARENT as i32);
        let title_font = create_font((32.0 * scale).round() as i32, true, language);
        let body_font = create_font((20.0 * scale).round() as i32, false, language);

        let title = match language {
            AppLanguage::English => "System Overlay",
            AppLanguage::SimplifiedChinese => "系统快捷面板",
        };
        let body = match language {
            AppLanguage::English => "Press B to close",
            AppLanguage::SimplifiedChinese => "按 B 键关闭",
        };

        let title_rect = RECT {
            left: left + (40.0 * scale).round() as i32,
            top: top + (118.0 * scale).round() as i32,
            right: right - (40.0 * scale).round() as i32,
            bottom: top + (165.0 * scale).round() as i32,
        };
        let body_rect = RECT {
            left: left + (40.0 * scale).round() as i32,
            top: top + (174.0 * scale).round() as i32,
            right: right - (40.0 * scale).round() as i32,
            bottom: bottom - (34.0 * scale).round() as i32,
        };

        SetTextColor(hdc, rgb(248, 250, 252));
        let old_font = SelectObject(hdc, title_font as HGDIOBJ);
        draw_text(
            hdc,
            title,
            title_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        SelectObject(hdc, old_font);

        SetTextColor(hdc, rgb(196, 206, 220));
        let old_font = SelectObject(hdc, body_font as HGDIOBJ);
        draw_text(hdc, body, body_rect, DT_CENTER | DT_WORDBREAK);
        SelectObject(hdc, old_font);

        DeleteObject(title_font as HGDIOBJ);
        DeleteObject(body_font as HGDIOBJ);
    }

    unsafe fn create_font(
        height: i32,
        bold: bool,
        language: AppLanguage,
    ) -> winapi::shared::windef::HFONT {
        let face = match language {
            AppLanguage::English => "Segoe UI",
            AppLanguage::SimplifiedChinese => "Microsoft YaHei UI",
        };
        CreateFontW(
            -height,
            0,
            0,
            0,
            if bold { FW_BOLD } else { FW_NORMAL },
            FALSE as u32,
            FALSE as u32,
            FALSE as u32,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH | FF_SWISS,
            wide(face).as_ptr(),
        )
    }

    unsafe fn draw_text(
        hdc: winapi::shared::windef::HDC,
        text: &str,
        mut rect: RECT,
        format: UINT,
    ) {
        let wide_text = wide(text);
        DrawTextW(hdc, wide_text.as_ptr(), -1, &mut rect, format);
    }

    fn rgb(red: u8, green: u8, blue: u8) -> u32 {
        red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(not(target_os = "windows"))]
mod windows {
    use crate::i18n::AppLanguage;

    pub(super) fn start(_language: AppLanguage) {}
    pub(super) fn set_language(_language: AppLanguage) {}
    pub(super) fn set_visible(_visible: bool) {}
}

pub(crate) fn start(language: AppLanguage) {
    windows::start(language);
}

pub(crate) fn set_language(language: AppLanguage) {
    windows::set_language(language);
}

pub(crate) fn set_visible(visible: bool) {
    windows::set_visible(visible);
}
