//! 托盘：左键截图；菜单：截图、退出。

use std::time::Duration;

use tauri::{
    AppHandle,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::overlay;

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

pub fn setup(app: &AppHandle, hotkey_ok: bool) -> tauri::Result<()> {
    let label = if hotkey_ok { "截图\tAlt+A" } else { "截图（Alt+A 被占用）" };
    let capture = MenuItem::with_id(app, "capture", label, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&capture, &PredefinedMenuItem::separator(app)?, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(icon())
        .tooltip("Shard 截图")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "capture" => capture_later(app),
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
