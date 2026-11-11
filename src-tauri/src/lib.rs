mod commands;
mod image;
mod ocr;
mod overlay;
mod pin;
mod platform;
mod save;
mod tray;

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const HOTKEY: &str = "Alt+A";

pub fn run() {
    tauri::Builder::default()
        // 单实例必须最先注册：再次启动程序时直接开始截图
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| overlay::start(app)))
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
        .manage(overlay::CaptureState::default())
        .manage(pin::Pins::default())
        .setup(|app| {
            let handle = app.handle();
            overlay::setup(handle);
            let hotkey_ok = register_hotkey(handle);
            tray::setup(handle, hotkey_ok)?;
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
        ])
        .run(tauri::generate_context!())
        .expect("启动 Shard 失败");
}

/// 注册截图热键；失败（多半是微信占用了 Alt+A）时弹窗提示
fn register_hotkey(app: &AppHandle) -> bool {
    match app.global_shortcut().register(HOTKEY) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("[shard] 注册 {HOTKEY} 失败：{e}");
            app.dialog()
                .message(format!(
                    "截图快捷键 {HOTKEY} 注册失败，可能被其他程序占用（微信的截图快捷键默认也是 {HOTKEY}）。\n\n\
                     请在微信「设置 → 快捷键」里改掉或关闭截图快捷键，然后重启 Shard。\n\
                     在此之前可以点托盘图标截图。"
                ))
                .title("Shard")
                .kind(MessageDialogKind::Warning)
                .show(|_| {});
            false
        }
    }
}
