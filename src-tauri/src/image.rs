//! 图片在 Rust 内部和前后端之间的表示。
//! 前后端传输格式：`[宽 u32 LE][高 u32 LE][RGBA 像素...]`，不做 PNG/base64 编码，省掉编解码时间。

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// 自上而下逐行，每像素 RGBA
    pub rgba: Vec<u8>,
}

const HEADER: usize = 8;

impl Image {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < HEADER {
            return Err("图片数据不完整".into());
        }
        let width = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let height = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let len = width as usize * height as usize * 4;
        if width == 0 || height == 0 || bytes.len() != HEADER + len {
            return Err(format!("图片数据大小不对：{width}×{height}，{} 字节", bytes.len()));
        }
        Ok(Self { width, height, rgba: bytes[HEADER..].to_vec() })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER + self.rgba.len());
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.rgba);
        out
    }

    pub fn to_bgra(&self) -> Vec<u8> {
        let mut out = self.rgba.clone();
        for px in out.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        out
    }

    pub fn to_png(&self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, self.width, self.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut writer = enc.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&self.rgba).map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())?;
        Ok(out)
    }

    /// 剪贴板 CF_DIB：BITMAPINFOHEADER + 自下而上的 32 位 BGRA 像素
    pub fn to_dib(&self) -> Vec<u8> {
        let (w, h) = (self.width as usize, self.height as usize);
        let mut out = Vec::with_capacity(40 + w * h * 4);
        out.extend_from_slice(&40u32.to_le_bytes()); // biSize
        out.extend_from_slice(&(w as i32).to_le_bytes()); // biWidth
        out.extend_from_slice(&(h as i32).to_le_bytes()); // biHeight > 0：自下而上
        out.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
        out.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
        out.extend_from_slice(&0u32.to_le_bytes()); // biCompression = BI_RGB
        out.extend_from_slice(&((w * h * 4) as u32).to_le_bytes()); // biSizeImage
        out.extend_from_slice(&[0u8; 16]); // 分辨率、调色板：不用
        for row in self.rgba.chunks_exact(w * 4).rev() {
            for px in row.chunks_exact(4) {
                out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Image {
        // 2×2：第一行 红、绿，第二行 蓝、白
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
        Image { width: 2, height: 2, rgba }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let img = sample();
        let back = Image::decode(&img.encode()).unwrap();
        assert_eq!((back.width, back.height), (2, 2));
        assert_eq!(back.rgba, img.rgba);
    }

    #[test]
    fn decode_rejects_bad_size() {
        let mut bytes = sample().encode();
        bytes.pop();
        assert!(Image::decode(&bytes).is_err());
        assert!(Image::decode(&[1, 0, 0, 0]).is_err());
    }

    #[test]
    fn to_bgra_swaps_red_and_blue() {
        let img = Image { width: 1, height: 1, rgba: vec![10, 20, 30, 255] };
        assert_eq!(img.to_bgra(), vec![30, 20, 10, 255]);
    }

    #[test]
    fn dib_is_bottom_up_bgra() {
        let dib = sample().to_dib();
        assert_eq!(dib.len(), 40 + 16);
        assert_eq!(i32::from_le_bytes(dib[8..12].try_into().unwrap()), 2);
        // 第一行像素是原图最后一行：蓝、白
        assert_eq!(&dib[40..48], &[255, 0, 0, 255, 255, 255, 255, 255]);
        // 第二行是原图第一行：红、绿
        assert_eq!(&dib[48..56], &[0, 0, 255, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn png_has_signature() {
        let png = sample().to_png().unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
}
