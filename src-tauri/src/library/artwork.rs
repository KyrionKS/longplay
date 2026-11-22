//! Cover extraction and cache.
//!
//! Covers are resized once and stored as JPEG in the app cache, so the webview
//! never has to load multi-megabyte artwork.

use std::fs;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;

pub const COVER_SIZE: u32 = 512;
const COVER_NAMES: &[&str] = &[
    "cover.jpg",
    "cover.jpeg",
    "cover.png",
    "folder.jpg",
    "folder.jpeg",
    "folder.png",
    "front.jpg",
    "front.png",
    "Front.jpg",
];

pub fn find_folder_cover(folder: &Path) -> Option<PathBuf> {
    COVER_NAMES
        .iter()
        .map(|name| folder.join(name))
        .find(|path| path.is_file())
}

pub fn bytes(embedded: Option<&[u8]>, folder_cover: Option<&Path>) -> Option<Vec<u8>> {
    embedded
        .map(ToOwned::to_owned)
        .or_else(|| folder_cover.and_then(|path| fs::read(path).ok()))
}

pub fn resize_cover(source: &[u8], max: u32) -> Option<Vec<u8>> {
    let image = image::load_from_memory(source).ok()?;
    let resized = image.resize(max, max, FilterType::Lanczos3).to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 82)
        .encode_image(&resized)
        .ok()?;
    Some(out)
}

pub fn cache(
    embedded: Option<&[u8]>,
    folder_cover: Option<&Path>,
    album_id: &str,
    cache_dir: &Path,
) -> Option<String> {
    let source = bytes(embedded, folder_cover)?;
    let resized = resize_cover(&source, COVER_SIZE)?;
    fs::create_dir_all(cache_dir).ok()?;
    let path = cache_dir.join(format!("{album_id}.jpg"));
    fs::write(&path, resized).ok()?;
    Some(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, RgbImage};
    use std::io::Cursor;

    fn png_bytes() -> Vec<u8> {
        let img = RgbImage::from_pixel(1024, 1024, image::Rgb([40, 90, 120]));
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn resizes_and_encodes_jpeg() {
        let bytes = resize_cover(&png_bytes(), 256).expect("cover");
        assert_eq!(&bytes[..2], &[0xff, 0xd8], "jpeg magic");
        assert!(bytes.len() < 100_000);
    }

    #[test]
    fn rejects_non_images() {
        assert!(resize_cover(b"not an image", 128).is_none());
    }
}
