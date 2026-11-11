//! 用 WebView2 SharedBuffer 把截图交给前端：BitBlt 直接写进共享内存，前端拿到的 ArrayBuffer 就是这块内存，
//! 不走 IPC 拷贝（4K 截图 33MB，走 IPC 要 300ms+）。前端用完调 chrome.webview.releaseBuffer 释放。

#[cfg(windows)]
pub fn post_screen(webview: &tauri::webview::PlatformWebview, b: super::screen::Bounds, session: u32) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_SHARED_BUFFER_ACCESS_READ_ONLY, ICoreWebView2_2, ICoreWebView2_17, ICoreWebView2Environment12,
    };
    use windows::Win32::Foundation::HANDLE;
    use windows_core::{HSTRING, Interface, PCWSTR};

    let run = || -> windows_core::Result<Result<(), String>> {
        unsafe {
            let core = webview.controller().CoreWebView2()?;
            let env = core.cast::<ICoreWebView2_2>()?.Environment()?.cast::<ICoreWebView2Environment12>()?;
            let buffer = env.CreateSharedBuffer(b.w as u64 * b.h as u64 * 4)?;
            let mut section = HANDLE::default();
            buffer.FileMappingHandle(&mut section)?;
            if let Err(e) = super::screen::capture_into(section, b) {
                return Ok(Err(e));
            }
            let meta = HSTRING::from(format!(
                r#"{{"kind":"capture","width":{},"height":{},"session":{session}}}"#,
                b.w, b.h
            ));
            core.cast::<ICoreWebView2_17>()?.PostSharedBufferToScript(
                &buffer,
                COREWEBVIEW2_SHARED_BUFFER_ACCESS_READ_ONLY,
                PCWSTR(meta.as_ptr()),
            )?;
            Ok(Ok(()))
        }
    };
    run().map_err(|e| format!("传递截图失败：{}", e.message()))?
}

#[cfg(not(windows))]
pub fn post_screen(_webview: &tauri::webview::PlatformWebview, _b: super::screen::Bounds, _session: u32) -> Result<(), String> {
    Err("只支持 Windows".into())
}
