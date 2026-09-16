//! F12 图片：剪贴板位图 → PNG 原图（`images/`）+ 缩略图（`thumbs/`）。
//!
//! 文件不塞进 SQLite，库里只存路径（`image_path` / `thumb_path`），
//! 这与规格书第 5 节的约定一致。

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};

use crate::error::{Error, Result};

/// 缩略图边长上限（F12：256px）。等比缩放，不裁切。
pub const THUMB_MAX: u32 = 256;

/// 把一张 RGBA 位图落成 PNG 原图 + 缩略图，返回两个文件路径。
///
/// `data_dir` 是应用数据目录（安装目录下的 `data/`），两个子目录在这里创建。
/// 调用方已经算过哈希的路径应该用 `save_image_bytes`，避免二次编码。
#[allow(dead_code)]
pub fn save_image(
    data_dir: &Path,
    img: &arboard::ImageData<'_>,
    hash: &str,
) -> Result<(PathBuf, PathBuf)> {
    let png_bytes = encode_png(img)?;
    save_image_bytes(data_dir, img, &png_bytes, hash)
}

/// 同上，但 PNG 字节由调用方提供 —— `monitor` 算哈希时已经编码过一次，
/// 传下来就不必再编一遍。
pub fn save_image_bytes(
    data_dir: &Path,
    img: &arboard::ImageData<'_>,
    png_bytes: &[u8],
    hash: &str,
) -> Result<(PathBuf, PathBuf)> {
    let images_dir = data_dir.join("images");
    let thumbs_dir = data_dir.join("thumbs");
    std::fs::create_dir_all(&images_dir)?;
    std::fs::create_dir_all(&thumbs_dir)?;

    let image_path = images_dir.join(format!("{hash}.png"));
    std::fs::write(&image_path, png_bytes)?;

    let thumb = make_thumb(img);
    let thumb_path = thumbs_dir.join(format!("{hash}.png"));
    let thumb_bytes = encode_png(&thumb)?;
    std::fs::write(&thumb_path, &thumb_bytes)?;

    Ok((image_path, thumb_path))
}

/// RGBA → PNG 字节。arboard 给的是 `width * height * 4` 的裸 RGBA。
///
/// `pub(crate)` 是因为 monitor 要先拿 PNG 字节算哈希再决定要不要落盘。
pub(crate) fn encode_png_pub(img: &arboard::ImageData<'_>) -> Result<Vec<u8>> {
    encode_png(img)
}

fn encode_png(img: &arboard::ImageData<'_>) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let encoder = PngEncoder::new(&mut out);
    encoder
        .write_image(
            &img.bytes,
            img.width as u32,
            img.height as u32,
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| Error::Other(format!("PNG 编码失败：{e}")))?;
    Ok(out)
}

/// 生成等比缩略图。小图（≤256px）原样返回，不做放大。
fn make_thumb<'a>(img: &arboard::ImageData<'a>) -> arboard::ImageData<'a> {
    let w = img.width as u32;
    let h = img.height as u32;

    if w <= THUMB_MAX && h <= THUMB_MAX {
        return arboard::ImageData {
            width: img.width,
            height: img.height,
            bytes: Cow::Owned(img.bytes.to_vec()),
        };
    }

    // 手写一次盒式降采样 —— `image` crate 的 `thumbnail()` 走 DynamicImage，
    // 多一次拷贝；原始 RGBA 直接盒采样更省。4×4 像素块平均（缩到 1/4 以内
    // 时块更大），对剪贴板缩略图足够。
    let scale = (w.max(h) + THUMB_MAX - 1) / THUMB_MAX;
    let tw = (w / scale).max(1);
    let th = (h / scale).max(1);

    let mut out = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        for tx in 0..tw {
            let mut acc = [0u64; 4];
            let mut n = 0u64;
            let x0 = tx * scale;
            let y0 = ty * scale;
            let x1 = (x0 + scale).min(w);
            let y1 = (y0 + scale).min(h);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = ((y * w + x) * 4) as usize;
                    acc[0] += img.bytes[i] as u64;
                    acc[1] += img.bytes[i + 1] as u64;
                    acc[2] += img.bytes[i + 2] as u64;
                    acc[3] += img.bytes[i + 3] as u64;
                    n += 1;
                }
            }
            let o = ((ty * tw + tx) * 4) as usize;
            out[o] = (acc[0] / n) as u8;
            out[o + 1] = (acc[1] / n) as u8;
            out[o + 2] = (acc[2] / n) as u8;
            out[o + 3] = (acc[3] / n) as u8;
        }
    }

    arboard::ImageData {
        width: tw as usize,
        height: th as usize,
        bytes: Cow::Owned(out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: usize, h: usize, rgba: [u8; 4]) -> arboard::ImageData<'static> {
        let mut bytes = Vec::with_capacity(w * h * 4);
        for _ in 0..w * h {
            bytes.extend_from_slice(&rgba);
        }
        arboard::ImageData {
            width: w,
            height: h,
            bytes: Cow::Owned(bytes),
        }
    }

    #[test]
    fn 小图缩略图不放大() {
        let img = solid(100, 80, [255, 0, 0, 255]);
        let thumb = make_thumb(&img);
        assert_eq!((thumb.width, thumb.height), (100, 80));
    }

    #[test]
    fn 大图缩略图等比缩小() {
        let img = solid(1024, 512, [0, 255, 0, 255]);
        let thumb = make_thumb(&img);
        assert_eq!((thumb.width, thumb.height), (256, 128));
    }

    #[test]
    fn 编码出合法png() {
        let img = solid(8, 8, [18, 165, 148, 255]);
        let png = encode_png(&img).expect("PNG 编码失败");
        // PNG 魔数
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn 落盘生成两个文件() {
        let dir = std::env::temp_dir().join(format!(
            "plico-img-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let img = solid(64, 64, [0, 0, 255, 255]);
        let (image_path, thumb_path) =
            save_image(&dir, &img, "deadbeef").expect("落盘失败");

        assert!(image_path.exists(), "原图应该存在");
        assert!(thumb_path.exists(), "缩略图应该存在");
        assert!(image_path.starts_with(dir.join("images")));
        assert!(thumb_path.starts_with(dir.join("thumbs")));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
