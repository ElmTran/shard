//! 截取整个虚拟屏幕（所有显示器）。进程是 Per-Monitor V2 DPI 感知，拿到的是物理像素。
//! 像素直接写进调用方给的共享内存（WebView2 SharedBuffer 的文件映射），不经过任何拷贝。

/// 虚拟屏幕在屏幕坐标里的范围（多显示器时左上角可能为负）
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[cfg(windows)]
pub fn virtual_screen() -> Bounds {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };
    unsafe {
        Bounds {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            w: GetSystemMetrics(SM_CXVIRTUALSCREEN),
            h: GetSystemMetrics(SM_CYVIRTUALSCREEN),
        }
    }
}

/// 把屏幕 b 范围内的像素以自上而下、32 位 BGRX（alpha 无意义）写进文件映射 section 的开头。
/// section 至少要有 w*h*4 字节
#[cfg(windows)]
pub fn capture_into(section: windows::Win32::Foundation::HANDLE, b: Bounds) -> Result<(), String> {
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
        DeleteDC, DeleteObject, GetDC, ReleaseDC, SRCCOPY, SelectObject,
    };

    if b.w <= 0 || b.h <= 0 {
        return Err("获取屏幕尺寸失败".into());
    }
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: b.w,
                biHeight: -b.h, // 负数：自上而下
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let result = match CreateDIBSection(Some(screen), &info, DIB_RGB_COLORS, &mut bits, Some(section), 0) {
            Ok(dib) => {
                let old = SelectObject(mem, dib.into());
                // 不加 CAPTUREBLT：DWM 下所有窗口（包括分层窗口）本来就合成在屏幕里，加了反而慢
                let blt = BitBlt(mem, 0, 0, b.w, b.h, Some(screen), b.x, b.y, SRCCOPY);
                SelectObject(mem, old);
                let _ = DeleteObject(dib.into());
                blt.map_err(|e| format!("截屏失败：{e}"))
            }
            Err(e) => Err(format!("创建位图失败：{e}")),
        };
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        result
    }
}

#[cfg(not(windows))]
pub fn virtual_screen() -> Bounds {
    Bounds { x: 0, y: 0, w: 0, h: 0 }
}
