//! 贴图：每个贴图一个窗口（pin-N），置顶、可拖动。图片存在 Rust 侧，复制/保存/OCR 直接用。
//!
//! 新建 WebView 窗口要几百毫秒，所以始终预备一个隐藏的、页面已加载好的备用窗口：
//! 贴图时把图片交给它、摆好位置就能显示，然后在后台再预备下一个。
//! 窗口四周留一圈透明边距，阴影由前端用 CSS 画（比系统阴影柔和）。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::image::Image;
use crate::platform::window;

/// 图片四周的透明边距（逻辑像素），给 CSS 阴影留地方。前端 .pin 的 padding 要和它一致
pub const PAD: f64 = 16.0;

#[derive(Default)]
struct Spare {
    label: Option<String>,
    /// 备用窗口的页面已经加载好，在等图片
    ready: bool,
}

#[derive(Default)]
pub struct Pins {
    next: AtomicU32,
    images: Mutex<HashMap<String, Image>>,
    spare: Mutex<Spare>,
}

impl Pins {
    pub fn with_image<T>(&self, label: &str, f: impl FnOnce(&Image) -> T) -> Result<T, String> {
        let images = self.images.lock().unwrap();
        images.get(label).map(f).ok_or_else(|| "贴图已关闭".into())
    }
}

fn build_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    let pins = app.state::<Pins>();
    let label = format!("pin-{}", pins.next.fetch_add(1, Ordering::Relaxed));
    let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("src/pin/index.html".into()))
        .title("Shard 贴图")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .shadow(false)
        // 透明窗口：边距里画阴影，关闭时可以淡出
        .transparent(true)
        .visible(false)
        .focused(false)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .build()
        .map_err(|e| format!("创建贴图失败：{e}"))?;
    window::disable_transitions(&win);
    window::square_corners(&win);

    let handle = app.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::Destroyed = event {
            handle.state::<Pins>().images.lock().unwrap().remove(&label);
        }
    });
    Ok(win)
}

/// 在后台预备一个备用窗口（已有就不重复建）
pub fn prepare_spare(app: &AppHandle) {
    let pins = app.state::<Pins>();
    let mut spare = pins.spare.lock().unwrap();
    if spare.label.is_some() {
        return;
    }
    match build_window(app) {
        Ok(win) => *spare = Spare { label: Some(win.label().to_string()), ready: false },
        Err(e) => eprintln!("[shard] {e}"),
    }
}

/// 在屏幕坐标 (x, y)（物理像素）显示贴图，图片按 1:1 像素显示。前端画好后再 present/reveal
pub fn create(app: &AppHandle, image: Image, x: i32, y: i32) -> Result<(), String> {
    let pins = app.state::<Pins>();
    let (w, h) = (image.width, image.height);

    // 拿备用窗口；没有就现建一个
    let mut spare = pins.spare.lock().unwrap();
    let taken = spare.label.take().and_then(|l| app.get_webview_window(&l).map(|w| (w, spare.ready)));
    spare.ready = false;
    let (win, ready) = match taken {
        Some(t) => t,
        None => (build_window(app)?, false),
    };
    let label = win.label().to_string();

    let pad = (PAD * win.scale_factor().unwrap_or(1.0)).round() as i32;
    let _ = win.set_size(PhysicalSize::new(w + 2 * pad as u32, h + 2 * pad as u32));
    // 按客户区对齐，图片正好落在原选区上
    let (x, y) = (x - pad, y - pad);
    let _ = win.set_position(PhysicalPosition::new(x, y));
    if let (Ok(inner), Ok(outer)) = (win.inner_position(), win.outer_position()) {
        if inner != outer {
            let _ = win.set_position(PhysicalPosition::new(x - (inner.x - outer.x), y - (inner.y - outer.y)));
        }
    }

    pins.images.lock().unwrap().insert(label.clone(), image);
    // 页面已经在等：通知它加载；还没加载完的话，它加载完调 pin_ready 时会发现图片已经到了
    if ready {
        let _ = app.emit_to(&label, "shard://pin-load", ());
    }
    drop(spare);

    // 下一个备用窗口晚一点再建，不和这次显示抢资源
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(800));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || prepare_spare(&handle));
    });
    Ok(())
}

// ---- 命令 ----

/// 贴图页面加载好了。返回 true 表示图片已经在了，可以直接加载；否则等 shard://pin-load
#[tauri::command]
pub fn pin_ready(window: WebviewWindow, pins: tauri::State<'_, Pins>) -> bool {
    let mut spare = pins.spare.lock().unwrap();
    if pins.images.lock().unwrap().contains_key(window.label()) {
        return true;
    }
    if spare.label.as_deref() == Some(window.label()) {
        spare.ready = true;
    }
    false
}

#[tauri::command]
pub fn get_pin_image(window: WebviewWindow, pins: tauri::State<'_, Pins>) -> Result<tauri::ipc::Response, String> {
    pins.with_image(window.label(), |img| tauri::ipc::Response::new(img.encode()))
}

#[tauri::command]
pub fn close_pin(window: WebviewWindow) {
    let _ = window.destroy();
}
