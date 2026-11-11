//! 窗口的 Win32 操作。
//!
//! 显示流程（截图窗口和贴图窗口共用）：前端画好内容 → `present`：DWM 隐身（cloak）后显示 →
//! 前端等两帧让 WebView2 真正把新内容合成出来 → `reveal`：取消隐身并聚焦。
//! 这样不会闪出上一次的截图或 WebView 的白底。显示/隐藏都走 Win32，不和 Tauri 的 show/hide 混用
//! （混用时 Tauri 记录的可见状态会不准，它的 hide() 可能什么都不做）。

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DWMWA_CLOAK, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        DwmSetWindowAttribute,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_TOPMOST, SW_HIDE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
        SetForegroundWindow, SetWindowPos, ShowWindow,
    };

    fn hwnd(win: &tauri::WebviewWindow) -> Option<HWND> {
        win.hwnd().ok().map(|h| HWND(h.0 as _))
    }

    pub fn raw_hwnd(win: &tauri::WebviewWindow) -> isize {
        hwnd(win).map(|h| h.0 as isize).unwrap_or(0)
    }

    fn set_cloak(hwnd: HWND, cloak: bool) {
        let value: i32 = cloak.into();
        let _ = unsafe {
            DwmSetWindowAttribute(hwnd, DWMWA_CLOAK, &value as *const _ as *const _, size_of::<i32>() as u32)
        };
    }

    /// 设置位置和大小（物理像素），不改变显示状态和层级
    pub fn set_bounds(win: &tauri::WebviewWindow, x: i32, y: i32, w: i32, h: i32) {
        let Some(hwnd) = hwnd(win) else { return };
        let _ = unsafe { SetWindowPos(hwnd, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE) };
    }

    /// 置顶显示；cloak=true 时先隐身，等 reveal 再露出来
    pub fn present(win: &tauri::WebviewWindow, cloak: bool) {
        let Some(hwnd) = hwnd(win) else { return };
        if cloak {
            set_cloak(hwnd, true);
        }
        let _ = unsafe {
            SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW)
        };
    }

    /// 取消隐身并聚焦
    pub fn reveal(win: &tauri::WebviewWindow) {
        let Some(hwnd) = hwnd(win) else { return };
        set_cloak(hwnd, false);
        let _ = unsafe { SetForegroundWindow(hwnd) };
        let _ = win.set_focus();
    }

    pub fn hide(win: &tauri::WebviewWindow) {
        let Some(hwnd) = hwnd(win) else { return };
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
    }

    /// 关掉系统的显示/隐藏动画（Win11 会缩放淡出，全屏截图窗口看起来就像“闪退”）。过渡由前端自己做
    pub fn disable_transitions(win: &tauri::WebviewWindow) {
        let Some(hwnd) = hwnd(win) else { return };
        let disabled: i32 = 1;
        let _ = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &disabled as *const _ as *const _,
                size_of::<i32>() as u32,
            )
        };
    }

    /// 贴图窗口保持直角，不让 Win11 裁掉图片的四个角
    pub fn square_corners(win: &tauri::WebviewWindow) {
        let Some(hwnd) = hwnd(win) else { return };
        let pref = DWMWCP_DONOTROUND;
        let _ = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &pref as *const _ as *const _,
                size_of_val(&pref) as u32,
            )
        };
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn raw_hwnd(_win: &tauri::WebviewWindow) -> isize {
        0
    }
    pub fn set_bounds(win: &tauri::WebviewWindow, x: i32, y: i32, w: i32, h: i32) {
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = win.set_size(tauri::PhysicalSize::new(w as u32, h as u32));
    }
    pub fn present(win: &tauri::WebviewWindow, _cloak: bool) {
        let _ = win.show();
    }
    pub fn reveal(win: &tauri::WebviewWindow) {
        let _ = win.set_focus();
    }
    pub fn hide(win: &tauri::WebviewWindow) {
        let _ = win.hide();
    }
    pub fn square_corners(_win: &tauri::WebviewWindow) {}
    pub fn disable_transitions(_win: &tauri::WebviewWindow) {}
}

pub use imp::*;
