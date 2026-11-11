//! 托盘：左键截图；菜单：截图（显示当前快捷键）、设置、退出。

use std::time::Duration;

use tauri::{
    AppHandle, Manager, Wry,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::{config, hotkey, overlay, settings};

/// “截图”菜单项，快捷键变化时更新文字
struct CaptureItem(MenuItem<Wry>);

pub fn icon() -> tauri::image::Image<'static> {
    tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png")).expect("tray.png 不是有效的 PNG")
}

/// 等托盘菜单/弹出层消失后再截，免得把它们截进去
fn capture_later(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || overlay::start(&handle));
    });
}

fn capture_label(app: &AppHandle) -> String {
    let key = config::get(app).hotkey;
    if hotkey::is_registered(app) { format!("截图\t{key}") } else { format!("截图（{key} 未生效）") }
}

pub fn sync_hotkey(app: &AppHandle) {
    if let Some(item) = app.try_state::<CaptureItem>() {
        let _ = item.0.set_text(capture_label(app));
    }
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let capture = MenuItem::with_id(app, "capture", capture_label(app), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&capture, &settings, &PredefinedMenuItem::separator(app)?, &quit])?;
    app.manage(CaptureItem(capture));

    TrayIconBuilder::with_id("main")
        .icon(icon())
        .tooltip("Shard 截图")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "capture" => capture_later(app),
            "settings" => settings::open(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                capture_later(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
