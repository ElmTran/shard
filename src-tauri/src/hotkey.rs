//! 截图快捷键：注册、更换（失败时恢复原来的）、录入新快捷键时暂停。

use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::config;

/// 当前注册着的快捷键；暂停或注册失败时为 None
#[derive(Default)]
pub struct Hotkey(Mutex<Option<Shortcut>>);

fn parse(s: &str) -> Result<Shortcut, String> {
    s.parse::<Shortcut>().map_err(|_| format!("快捷键「{s}」格式不对"))
}

/// 换成 wanted。注册失败时恢复原来的，并返回错误
pub fn apply(app: &AppHandle, wanted: &str) -> Result<(), String> {
    let new = parse(wanted)?;
    let state = app.state::<Hotkey>();
    let mut current = state.0.lock().unwrap();
    if *current == Some(new) {
        return Ok(());
    }
    let gs = app.global_shortcut();
    let old = current.take();
    if let Some(old) = old {
        let _ = gs.unregister(old);
    }
    match gs.register(new) {
        Ok(()) => {
            *current = Some(new);
            Ok(())
        }
        Err(_) => {
            if let Some(old) = old {
                if gs.register(old).is_ok() {
                    *current = Some(old);
                }
            }
            Err(format!("「{wanted}」注册失败，可能被其他程序占用"))
        }
    }
}

/// 设置窗口正在录入快捷键：先注销，否则按下要录的组合键会直接开始截图
pub fn set_suspended(app: &AppHandle, suspended: bool) -> Result<(), String> {
    if suspended {
        if let Some(s) = app.state::<Hotkey>().0.lock().unwrap().take() {
            let _ = app.global_shortcut().unregister(s);
        }
        Ok(())
    } else {
        apply(app, &config::get(app).hotkey)
    }
}

pub fn is_registered(app: &AppHandle) -> bool {
    app.state::<Hotkey>().0.lock().unwrap().is_some()
}
