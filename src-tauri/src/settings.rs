//! 设置窗口（按需创建，关闭即销毁）和它的命令：快捷键、开机自启。

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

use crate::{config, hotkey, tray};

pub const SETTINGS: &str = "settings";

pub fn open(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(SETTINGS) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("src/settings/index.html".into()))
        .title("Shard 设置")
        .icon(tray::icon())
        .and_then(|b| {
            b.inner_size(420.0, 196.0)
                .resizable(false)
                .maximizable(false)
                .minimizable(false)
                .center()
                // 透明 + Mica：Win11 原生背景，打开时也不会白屏一闪
                .transparent(true)
                .effects(tauri::utils::config::WindowEffectsConfig {
                    effects: vec![tauri::window::Effect::Mica],
                    ..Default::default()
                })
                .build()
        });
    match built {
        Ok(win) => {
            let handle = app.clone();
            win.on_window_event(move |event| {
                // 录入快捷键时直接关窗口：恢复快捷键
                if let WindowEvent::Destroyed = event {
                    let _ = hotkey::set_suspended(&handle, false);
                }
            });
        }
        Err(e) => eprintln!("[shard] 打开设置窗口失败：{e}"),
    }
}

#[derive(Serialize)]
pub struct Settings {
    hotkey: String,
    /// 快捷键当前是否生效（可能被其他程序占用）
    registered: bool,
    autostart: bool,
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    Settings {
        hotkey: config::get(&app).hotkey,
        registered: hotkey::is_registered(&app),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

/// 换快捷键：先试注册，成功了才保存
#[tauri::command]
pub fn set_hotkey(app: AppHandle, hotkey: String) -> Result<(), String> {
    hotkey::apply(&app, &hotkey)?;
    config::update(&app, |c| c.hotkey = hotkey.clone())?;
    tray::sync_hotkey(&app);
    Ok(())
}

#[tauri::command]
pub fn suspend_hotkey(app: AppHandle, suspended: bool) -> Result<(), String> {
    let result = hotkey::set_suspended(&app, suspended);
    tray::sync_hotkey(&app);
    result
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| format!("设置开机启动失败：{e}"))
}
