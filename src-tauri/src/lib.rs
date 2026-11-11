mod commands;
mod config;
mod hotkey;
mod image;
mod ocr;
mod overlay;
mod pin;
mod platform;
mod save;
mod settings;
mod tray;

use tauri::AppHandle;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_global_shortcut::ShortcutState;

pub fn run() {
    tauri::Builder::default()
        // 单实例必须最先注册：再次启动程序时直接开始截图
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| overlay::start(app)))
        // 只注册截图一个快捷键，按下就截图
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        overlay::start(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        // 开机自启写在注册表 Run 键里，状态以注册表为准
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .manage(overlay::CaptureState::default())
        .manage(pin::Pins::default())
        .manage(hotkey::Hotkey::default())
        .setup(|app| {
            let handle = app.handle();
            config::init(handle);
            overlay::setup(handle);
            register_hotkey(handle);
            tray::setup(handle)?;
            pin::prepare_spare(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::present,
            commands::reveal,
            commands::overlay_copy,
            commands::overlay_pin,
            commands::overlay_save,
            commands::overlay_ocr,
            commands::copy_text,
            commands::pin_copy,
            commands::pin_save,
            commands::pin_ocr,
            overlay::close_overlay,
            pin::pin_ready,
            pin::get_pin_image,
            pin::close_pin,
            settings::get_settings,
            settings::set_hotkey,
            settings::suspend_hotkey,
            settings::set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Shard 失败");
}

/// 注册截图快捷键；失败（多半被微信、PixPin 等占用）时弹窗提示，并打开设置窗口方便换一个
fn register_hotkey(app: &AppHandle) {
    let key = config::get(app).hotkey;
    if let Err(e) = hotkey::apply(app, &key) {
        eprintln!("[shard] {e}");
        app.dialog()
            .message(format!(
                "截图快捷键 {key} 注册失败，可能被其他程序占用（微信、PixPin 的截图快捷键默认也是 Alt+A）。\n\n\
                 可以在 Shard 设置里换一个快捷键，或者在对方的设置里改掉。在此之前可以点托盘图标截图。"
            ))
            .title("Shard")
            .kind(MessageDialogKind::Warning)
            .show({
                let app = app.clone();
                move |_| settings::open(&app)
            });
    }
}
