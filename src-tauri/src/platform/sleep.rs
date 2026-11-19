//! 隐藏的 WebView 休眠：降低内存目标并挂起（TrySuspend），WebView2 会把渲染进程的内存换出。
//! 挂起期间页面的 JS、定时器、事件都暂停，所以要在发事件/传截图给前端之前先唤醒。
//! 顺序有讲究：可见的 WebView 挂不起来，所以先 SetIsVisible(false)；唤醒时 SetIsVisible(true) 本身就会恢复。

#[cfg(windows)]
pub fn set_awake(win: &tauri::WebviewWindow, awake: bool) {
    let label = win.label().to_string();
    let posted = win.with_webview(move |webview| {
        if let Err(e) = unsafe { apply(&webview, awake, &label) } {
            eprintln!("[shard] {label} {}失败：{}", if awake { "唤醒" } else { "休眠" }, e.message());
        }
    });
    if let Err(e) = posted {
        eprintln!("[shard] {}：{e}", win.label());
    }
}

#[cfg(windows)]
unsafe fn apply(webview: &tauri::webview::PlatformWebview, awake: bool, label: &str) -> windows_core::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL, ICoreWebView2_3,
        ICoreWebView2_19,
    };
    use webview2_com::TrySuspendCompletedHandler;
    use windows_core::Interface;

    unsafe {
        let controller = webview.controller();
        controller.SetIsVisible(awake)?;
        let core = controller.CoreWebView2()?;

        // 内存目标是锦上添花，旧版运行时没有这个接口也继续挂起
        match core.cast::<ICoreWebView2_19>() {
            Ok(core) => {
                let level = if awake {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                } else {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                };
                if let Err(e) = core.SetMemoryUsageTargetLevel(level) {
                    eprintln!("[shard] {label} 设置内存目标失败：{}", e.message());
                }
            }
            Err(e) => eprintln!("[shard] {label} 不支持内存目标：{}", e.message()),
        }

        let core = core.cast::<ICoreWebView2_3>()?;
        if awake {
            core.Resume()
        } else {
            let label = label.to_owned();
            let handler = TrySuspendCompletedHandler::create(Box::new(move |result, suspended| {
                match result {
                    Ok(()) if suspended => {}
                    // 挂起完成前又被唤醒（变成可见）时就是这样，不算错
                    Ok(()) => eprintln!("[shard] {label} 没有挂起"),
                    Err(e) => eprintln!("[shard] {label} 挂起失败：{}", e.message()),
                }
                Ok(())
            }));
            core.TrySuspend(&handler)
        }
    }
}

#[cfg(not(windows))]
pub fn set_awake(_win: &tauri::WebviewWindow, _awake: bool) {}
