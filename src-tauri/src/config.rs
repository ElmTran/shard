//! 设置：保存在 %APPDATA%\com.shard.app\config.json。开机自启不在这里，状态以注册表为准。

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const DEFAULT_HOTKEY: &str = "Alt+A";

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 截图快捷键，格式同 tauri-plugin-global-shortcut，如 "Alt+A"、"Ctrl+Shift+X"、"F1"
    pub hotkey: String,
}

impl Default for Config {
    fn default() -> Self {
        Self { hotkey: DEFAULT_HOTKEY.into() }
    }
}

pub struct ConfigState(Mutex<Config>);

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

/// 读取配置；文件不存在或格式不对时用默认值
pub fn init(app: &AppHandle) {
    let cfg = path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    app.manage(ConfigState(Mutex::new(cfg)));
}

pub fn get(app: &AppHandle) -> Config {
    app.state::<ConfigState>().0.lock().unwrap().clone()
}

pub fn update(app: &AppHandle, f: impl FnOnce(&mut Config)) -> Result<(), String> {
    let state = app.state::<ConfigState>();
    let mut cfg = state.0.lock().unwrap();
    let mut next = cfg.clone();
    f(&mut next);
    let p = path(app).ok_or("找不到配置目录")?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("保存设置失败：{e}"))?;
    }
    let text = serde_json::to_string_pretty(&next).map_err(|e| e.to_string())?;
    std::fs::write(&p, text).map_err(|e| format!("保存设置失败：{e}"))?;
    *cfg = next;
    Ok(())
}
