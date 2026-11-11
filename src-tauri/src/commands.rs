//! 前端调用的命令：显示流程、复制、贴图、保存、OCR。
//! 截图窗口的动作带上合成好的图片（二进制请求体），执行前先关掉截图窗口；
//! 贴图窗口的动作直接用 Rust 侧存的图片。

use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::image::Image;
use crate::ocr::{self, OcrLine};
use crate::overlay::{self, OVERLAY};
use crate::pin::{self, Pins};
use crate::platform::{clipboard, window};
use crate::save;

fn body_image(request: &Request<'_>) -> Result<Image, String> {
    match request.body() {
        InvokeBody::Raw(bytes) => Image::decode(bytes),
        _ => Err("需要二进制图片数据".into()),
    }
}

fn header_i32s(request: &Request<'_>, name: &str) -> Result<Vec<i32>, String> {
    let value = request.headers().get(name).and_then(|v| v.to_str().ok()).ok_or(format!("缺少 {name}"))?;
    value.split(',').map(|s| s.trim().parse::<i32>().map_err(|e| e.to_string())).collect()
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

/// 截图窗口的动作出错时窗口已经关了，前端没处显示，弹个系统对话框
fn report<T>(app: &AppHandle, result: Result<T, String>) -> Result<T, String> {
    if let Err(e) = &result {
        app.dialog().message(e).title("Shard").kind(MessageDialogKind::Error).show(|_| {});
    }
    result
}

// ---- 显示流程（见 platform/window.rs）----

#[tauri::command]
pub fn present(window: WebviewWindow) {
    // 贴图是透明窗口，盖在截图窗口上面直接显示：没画出来之前是透明的，下面的截图窗口还在
    window::present(&window, window.label() == OVERLAY);
}

#[tauri::command]
pub fn reveal(app: AppHandle, window: WebviewWindow) {
    window::reveal(&window);
    if window.label() != OVERLAY {
        overlay::pin_shown(&app);
    }
}

// ---- 截图窗口 ----

#[tauri::command]
pub async fn overlay_copy(app: AppHandle, request: Request<'_>) -> Result<(), String> {
    let image = body_image(&request)?;
    overlay::close(&app);
    let owner = overlay::owner_hwnd(&app);
    let result = blocking(move || clipboard::set_image(owner, &image)).await;
    report(&app, result)
}

/// 请求头 x-pos：选区左上角相对截图的位置 "x,y"（物理像素）。
/// 截图窗口等贴图显示出来再隐藏（见 overlay::pin_shown）
#[tauri::command]
pub async fn overlay_pin(app: AppHandle, request: Request<'_>) -> Result<(), String> {
    let image = body_image(&request)?;
    let pos = header_i32s(&request, "x-pos")?;
    let [x, y] = pos[..] else { return Err("x-pos 格式不对".into()) };
    let (ox, oy) = overlay::origin(&app);
    overlay::begin_pin(&app);
    let result = pin::create(&app, image, ox + x, oy + y);
    if result.is_err() {
        overlay::close(&app);
    }
    report(&app, result)
}

#[tauri::command]
pub async fn overlay_save(app: AppHandle, request: Request<'_>) -> Result<bool, String> {
    let image = body_image(&request)?;
    // 截图窗口全屏置顶，会挡住对话框，先关掉
    overlay::close(&app);
    let handle = app.clone();
    let result = blocking(move || save::save_png(&handle, &image, None)).await;
    report(&app, result)
}

#[tauri::command]
pub async fn overlay_ocr(request: Request<'_>) -> Result<Vec<OcrLine>, String> {
    let image = body_image(&request)?;
    blocking(move || ocr::recognize(&image)).await
}

/// 复制文字。从截图窗口复制时顺便关掉截图窗口
#[tauri::command]
pub async fn copy_text(app: AppHandle, window: WebviewWindow, text: String) -> Result<(), String> {
    if window.label() == OVERLAY {
        overlay::close(&app);
    }
    let owner = overlay::owner_hwnd(&app);
    blocking(move || clipboard::set_text(owner, &text)).await
}

// ---- 贴图窗口 ----

fn pin_image(app: &AppHandle, window: &WebviewWindow) -> Result<Image, String> {
    app.state::<Pins>().with_image(window.label(), Image::clone)
}

#[tauri::command]
pub async fn pin_copy(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    let image = pin_image(&app, &window)?;
    let owner = overlay::owner_hwnd(&app);
    blocking(move || clipboard::set_image(owner, &image)).await
}

#[tauri::command]
pub async fn pin_save(app: AppHandle, window: WebviewWindow) -> Result<bool, String> {
    let image = pin_image(&app, &window)?;
    blocking(move || save::save_png(&app, &image, Some(&window))).await
}

#[tauri::command]
pub async fn pin_ocr(app: AppHandle, window: WebviewWindow) -> Result<Vec<OcrLine>, String> {
    let image = pin_image(&app, &window)?;
    blocking(move || ocr::recognize(&image)).await
}
