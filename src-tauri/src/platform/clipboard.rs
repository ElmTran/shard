//! 剪贴板：写图片（CF_DIB + PNG）和文本。打开剪贴板的重试逻辑来自 relic。
//! owner 是剪贴板所有者窗口的 HWND（用常驻的截图窗口），为空时 SetClipboardData 可能失败。

#[cfg(windows)]
mod imp {
    use std::time::Duration;

    use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
    use windows::core::w;

    use crate::image::Image;

    const CF_DIB: u32 = 8;
    const CF_UNICODETEXT: u32 = 13;

    /// 打开剪贴板期间持有，离开作用域时关闭
    struct Opened;

    impl Opened {
        /// 剪贴板可能正被别的程序占用，重试一会
        fn open(owner: isize) -> Result<Self, String> {
            let owner = Some(HWND(owner as _));
            for _ in 0..30 {
                if unsafe { OpenClipboard(owner) }.is_ok() {
                    return Ok(Opened);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err("剪贴板被其他程序占用".into())
        }
    }

    impl Drop for Opened {
        fn drop(&mut self) {
            let _ = unsafe { CloseClipboard() };
        }
    }

    /// 写入图片：CF_DIB 给微信、Office、画图等，PNG 给浏览器等认 PNG 的程序
    pub fn set_image(owner: isize, image: &Image) -> Result<(), String> {
        // 编码放在打开剪贴板之前，尽量缩短占用时间
        let dib = image.to_dib();
        let png = image.to_png()?;
        let _open = Opened::open(owner)?;
        unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
        if !unsafe { set_bytes(CF_DIB, &dib) } {
            return Err("写入剪贴板失败".into());
        }
        unsafe { set_bytes(RegisterClipboardFormatW(w!("PNG")), &png) };
        Ok(())
    }

    pub fn set_text(owner: isize, text: &str) -> Result<(), String> {
        let _open = Opened::open(owner)?;
        unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2) };
        if !unsafe { set_bytes(CF_UNICODETEXT, bytes) } {
            return Err("写入剪贴板失败".into());
        }
        Ok(())
    }

    /// 复制到新的 HGLOBAL 并放进剪贴板（成功后内存归系统所有）
    unsafe fn set_bytes(format: u32, bytes: &[u8]) -> bool {
        unsafe {
            let Ok(h) = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)) else { return false };
            let ptr = GlobalLock(h);
            if ptr.is_null() {
                let _ = GlobalFree(Some(h));
                return false;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), bytes.len());
            let _ = GlobalUnlock(h);
            if SetClipboardData(format, Some(HANDLE(h.0))).is_err() {
                let _ = GlobalFree(Some(h));
                return false;
            }
            true
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::image::Image;
    pub fn set_image(_owner: isize, _image: &Image) -> Result<(), String> {
        Err("只支持 Windows".into())
    }
    pub fn set_text(_owner: isize, _text: &str) -> Result<(), String> {
        Err("只支持 Windows".into())
    }
}

pub use imp::*;
