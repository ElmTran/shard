//! OCR：系统自带的 Windows.Media.Ocr，离线、不增加安装包体积。
//! 中文识别器会在每个汉字之间插空格（每个字是一个 word），拼行文本时按字符类型决定要不要空格。

use serde::Serialize;
use ts_rs::TS;

use crate::image::Image;

/// 一个词（中文通常是单个字）。坐标是图片像素，相对图片左上角
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct OcrWord {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct OcrLine {
    pub words: Vec<OcrWord>,
    /// 拼好的整行文本
    pub text: String,
}

/// 中日韩文字和全角标点：前后不加空格
fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x2E80..=0x2FDF   // 部首
        | 0x3000..=0x303F // 中文标点
        | 0x3040..=0x30FF // 假名
        | 0x3100..=0x312F // 注音
        | 0x3400..=0x4DBF // 扩展 A
        | 0x4E00..=0x9FFF // 基本汉字
        | 0xAC00..=0xD7AF // 谚文
        | 0xF900..=0xFAFF // 兼容汉字
        | 0xFE30..=0xFE4F // 竖排标点
        | 0xFF00..=0xFFEF // 全角字符
        | 0x20000..=0x3FFFF)
}

/// 把词拼成一行：两个词相邻处任一侧是中文就直接相连，否则用空格隔开
pub fn join_words<'a>(words: impl IntoIterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for word in words {
        if let (Some(prev), Some(next)) = (out.chars().last(), word.chars().next()) {
            if !is_cjk(prev) && !is_cjk(next) {
                out.push(' ');
            }
        }
        out.push_str(word);
    }
    out
}

#[cfg(windows)]
pub fn recognize(image: &Image) -> Result<Vec<OcrLine>, String> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Security::Cryptography::CryptographicBuffer;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    let max = OcrEngine::MaxImageDimension().map_err(|e| e.to_string())?;
    if image.width > max || image.height > max {
        return Err(format!("图片太大（最大 {max} 像素）"));
    }
    // 工作线程上可能还没初始化 COM；已经初始化过时返回 S_FALSE，忽略即可
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

    let run = || -> windows::core::Result<Vec<OcrLine>> {
        let engine = match OcrEngine::TryCreateFromUserProfileLanguages() {
            Ok(engine) => engine,
            // 用户语言没有对应的识别器时，用系统里装了的第一个
            Err(_) => {
                let lang = OcrEngine::AvailableRecognizerLanguages()?.GetAt(0)?;
                OcrEngine::TryCreateFromLanguage(&lang)?
            }
        };
        let buffer = CryptographicBuffer::CreateFromByteArray(&image.to_bgra())?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            image.width as i32,
            image.height as i32,
        )?;
        let result = engine.RecognizeAsync(&bitmap)?.join()?;

        let mut lines = Vec::new();
        for line in result.Lines()? {
            let mut words = Vec::new();
            for word in line.Words()? {
                let r = word.BoundingRect()?;
                words.push(OcrWord { text: word.Text()?.to_string(), x: r.X, y: r.Y, w: r.Width, h: r.Height });
            }
            if words.is_empty() {
                continue;
            }
            let text = join_words(words.iter().map(|w| w.text.as_str()));
            lines.push(OcrLine { words, text });
        }
        Ok(lines)
    };
    run().map_err(|e| format!("文字识别失败：{}", e.message()))
}

#[cfg(not(windows))]
pub fn recognize(_image: &Image) -> Result<Vec<OcrLine>, String> {
    Err("只支持 Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_chars_join_without_spaces() {
        assert_eq!(join_words(["截", "图", "工", "具"]), "截图工具");
    }

    #[test]
    fn latin_words_keep_spaces() {
        assert_eq!(join_words(["hello", "world"]), "hello world");
    }

    #[test]
    fn mixed_text_has_no_space_at_boundaries() {
        assert_eq!(join_words(["使", "用", "Tauri", "v2", "开", "发"]), "使用Tauri v2开发");
    }

    #[test]
    fn full_width_punctuation_joins() {
        assert_eq!(join_words(["好", "，", "OK", "。"]), "好，OK。");
    }

    #[test]
    fn empty_input() {
        assert_eq!(join_words([]), "");
    }
}
