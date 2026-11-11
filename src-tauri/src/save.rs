//! 保存为 PNG 文件：弹出系统保存对话框。

use tauri::{AppHandle, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::image::Image;

/// 默认文件名：shard_YYYYMMDD_HHMMSS.png（本地时间）
fn default_name() -> String {
    #[cfg(windows)]
    {
        let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        format!(
            "shard_{:04}{:02}{:02}_{:02}{:02}{:02}.png",
            t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
        )
    }
    #[cfg(not(windows))]
    {
        "shard.png".into()
    }
}

/// 阻塞直到对话框关闭；取消时返回 Ok(false)。parent 为 None 时对话框没有所有者窗口
pub fn save_png(app: &AppHandle, image: &Image, parent: Option<&WebviewWindow>) -> Result<bool, String> {
    let mut dialog = app.dialog().file().set_file_name(default_name()).add_filter("PNG 图片", &["png"]);
    if let Some(parent) = parent {
        dialog = dialog.set_parent(parent);
    }
    let Some(path) = dialog.blocking_save_file() else { return Ok(false) };
    let mut path = path.into_path().map_err(|e| e.to_string())?;
    if path.extension().is_none() {
        path.set_extension("png");
    }
    std::fs::write(&path, image.to_png()?).map_err(|e| format!("保存失败：{e}"))?;
    Ok(true)
}
