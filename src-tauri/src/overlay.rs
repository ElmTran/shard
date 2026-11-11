//! 截图窗口：启动时预建、常驻隐藏的全屏窗口（WebView 一直是热的，唤起快）。
//! 热键 → 窗口铺满虚拟屏幕（仍隐藏）→ 截屏直接写进 WebView2 共享内存并交给前端 →
//! 前端画好后 present/reveal（见 platform/window.rs）。Rust 侧不保留截图，导出时前端把合成好的选区发回来。

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use tauri::{AppHandle, Emitter, Manager, WebviewWindow, WindowEvent};

use crate::platform::{screen, shared, window};

pub const OVERLAY: &str = "overlay";

#[derive(Default)]
pub struct CaptureState {
    /// 本次截图左上角的屏幕坐标（贴图窗口定位用）
    origin: Mutex<(i32, i32)>,
    /// 截图窗口正在使用（从截屏到关闭），期间再按热键不重复截
    active: AtomicBool,
    /// 每次截图加一，前端用它判断“已隐藏”通知是不是针对当前这次
    session: AtomicU32,
    /// 正在从截图生成贴图：等贴图显示出来再隐藏截图窗口，中间不露出桌面
    pinning: AtomicBool,
}

pub fn overlay_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(OVERLAY)
}

/// 剪贴板所有者：常驻的截图窗口
pub fn owner_hwnd(app: &AppHandle) -> isize {
    overlay_window(app).map(|w| window::raw_hwnd(&w)).unwrap_or(0)
}

pub fn setup(app: &AppHandle) {
    let Some(win) = overlay_window(app) else { return };
    window::disable_transitions(&win);
    let handle = app.clone();
    win.on_window_event(move |event| {
        // Alt+F4：只隐藏，窗口常驻
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            close(&handle);
        }
    });
}

/// 开始截图（热键、托盘、再次启动程序）
pub fn start(app: &AppHandle) {
    let state = app.state::<CaptureState>();
    if state.active.swap(true, Ordering::AcqRel) {
        return;
    }
    let Some(win) = overlay_window(app) else {
        state.active.store(false, Ordering::Release);
        return;
    };
    state.pinning.store(false, Ordering::Release);
    let session = state.session.fetch_add(1, Ordering::AcqRel) + 1;
    let b = screen::virtual_screen();
    *state.origin.lock().unwrap() = (b.x, b.y);
    // 隐藏状态下先铺满虚拟屏幕，前端拿到截图时布局已经是对的
    window::set_bounds(&win, b.x, b.y, b.w, b.h);

    let handle = app.clone();
    let posted = win.with_webview(move |webview| {
        if let Err(e) = shared::post_screen(&webview, b, session) {
            eprintln!("[shard] {e}");
            handle.state::<CaptureState>().active.store(false, Ordering::Release);
        }
    });
    if let Err(e) = posted {
        eprintln!("[shard] {e}");
        state.active.store(false, Ordering::Release);
    }
}

/// 隐藏截图窗口，并通知前端释放这次截图
pub fn close(app: &AppHandle) {
    let state = app.state::<CaptureState>();
    if let Some(win) = overlay_window(app) {
        window::hide(&win);
        let _ = win.emit_to(OVERLAY, "shard://hidden", state.session.load(Ordering::Acquire));
    }
    state.pinning.store(false, Ordering::Release);
    state.active.store(false, Ordering::Release);
}

/// 贴图要从截图生成：先不隐藏截图窗口
pub fn begin_pin(app: &AppHandle) {
    app.state::<CaptureState>().pinning.store(true, Ordering::Release);
}

/// 贴图已经显示：再等一小会确保它真的画到屏幕上，然后隐藏截图窗口，看起来就是选区原地变成了贴图
pub fn pin_shown(app: &AppHandle) {
    if !app.state::<CaptureState>().pinning.load(Ordering::Acquire) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(60));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if handle.state::<CaptureState>().pinning.swap(false, Ordering::AcqRel) {
                close(&handle);
            }
        });
    });
}

pub fn origin(app: &AppHandle) -> (i32, i32) {
    *app.state::<CaptureState>().origin.lock().unwrap()
}

// ---- 命令 ----

#[tauri::command]
pub fn close_overlay(app: AppHandle) {
    close(&app);
}
