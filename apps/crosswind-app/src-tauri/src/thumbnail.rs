use std::io::Cursor;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::{ImageFormat, ImageReader};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

const THUMBNAIL_EDGE: u32 = 360;
const MAX_DECODED_FILE_BYTES: u64 = 60 * 1024 * 1024;
const JPEG_2000_MIME_TYPE: &str = "image/jp2";

#[derive(Clone, Serialize)]
struct CardThumbnail {
    thumbnail: String,
}

pub fn emit_card_thumbnail_for_file(app: &AppHandle, path: &Path) {
    if let Some(thumbnail) = data_url_for_file(path) {
        let _ = app.emit("card-thumbnail", CardThumbnail { thumbnail });
    }
}

pub fn data_url_for_file(path: &Path) -> Option<String> {
    thumbnail_from_decodable_image(path).or_else(|| system::thumbnail_from_any_file(path))
}

pub fn data_url_for_sender_icon(icon_bytes: &[u8]) -> Option<String> {
    match sniff_mime_type(icon_bytes)? {
        JPEG_2000_MIME_TYPE => system::png_from_other_image_format(icon_bytes),
        browser_ready_mime_type => Some(data_url(browser_ready_mime_type, icon_bytes)),
    }
}

fn sniff_mime_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.len() > 12 && &bytes[4..12] == b"jP  \r\n\x87\n" {
        Some(JPEG_2000_MIME_TYPE)
    } else {
        None
    }
}

fn thumbnail_from_decodable_image(path: &Path) -> Option<String> {
    if std::fs::metadata(path).ok()?.len() > MAX_DECODED_FILE_BYTES {
        return None;
    }
    let decoded = ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let thumbnail = decoded.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE);
    let mut png_bytes = Vec::new();
    thumbnail
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .ok()?;
    Some(data_url("image/png", &png_bytes))
}

fn data_url(mime_type: &str, bytes: &[u8]) -> String {
    format!("data:{mime_type};base64,{}", BASE64.encode(bytes))
}

#[cfg(target_os = "macos")]
mod system {
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{THUMBNAIL_EDGE, data_url};

    static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn create() -> Option<Self> {
            let dir = std::env::temp_dir().join(format!(
                "crosswind-thumbnail-{}-{}",
                std::process::id(),
                SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&dir).ok()?;
            Some(ScratchDir(dir))
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub fn thumbnail_from_any_file(path: &Path) -> Option<String> {
        let scratch = ScratchDir::create()?;
        let succeeded = Command::new("qlmanage")
            .args(["-t", "-s", &THUMBNAIL_EDGE.to_string(), "-o"])
            .arg(&scratch.0)
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?
            .success();
        if !succeeded {
            return None;
        }
        let thumbnail_name = format!("{}.png", path.file_name()?.to_string_lossy());
        let png_bytes = std::fs::read(scratch.0.join(thumbnail_name)).ok()?;
        Some(data_url("image/png", &png_bytes))
    }

    pub fn png_from_other_image_format(image_bytes: &[u8]) -> Option<String> {
        let scratch = ScratchDir::create()?;
        let source = scratch.0.join("icon");
        let converted = scratch.0.join("icon.png");
        std::fs::write(&source, image_bytes).ok()?;
        let succeeded = Command::new("sips")
            .args(["-s", "format", "png", "-Z", &THUMBNAIL_EDGE.to_string()])
            .arg(&source)
            .arg("--out")
            .arg(&converted)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?
            .success();
        if !succeeded {
            return None;
        }
        Some(data_url("image/png", &std::fs::read(converted).ok()?))
    }
}

#[cfg(not(target_os = "macos"))]
mod system {
    use std::path::Path;

    pub fn thumbnail_from_any_file(_path: &Path) -> Option<String> {
        None
    }

    pub fn png_from_other_image_format(_image_bytes: &[u8]) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_sample_png(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("crosswind-{}-{name}", std::process::id()));
        image::RgbImage::from_pixel(800, 600, image::Rgb([20, 120, 100]))
            .save_with_format(&path, ImageFormat::Png)
            .unwrap();
        path
    }

    #[test]
    fn image_file_becomes_a_small_png_data_url() {
        let path = write_sample_png("sample.png");
        let data_url = data_url_for_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(data_url.starts_with("data:image/png;base64,"));
        let png_bytes = BASE64.decode(data_url.trim_start_matches("data:image/png;base64,")).unwrap();
        let decoded = image::load_from_memory(&png_bytes).unwrap();
        assert!(decoded.width() <= THUMBNAIL_EDGE && decoded.height() <= THUMBNAIL_EDGE);
    }

    #[test]
    fn jpeg_sender_icon_is_used_as_is() {
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10];
        assert_eq!(
            data_url_for_sender_icon(&jpeg),
            Some(format!("data:image/jpeg;base64,{}", BASE64.encode(jpeg)))
        );
    }

    #[test]
    fn unknown_sender_icon_is_ignored() {
        assert_eq!(data_url_for_sender_icon(b"not an image"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn non_image_file_gets_a_quicklook_thumbnail() {
        let path = std::env::temp_dir().join(format!("crosswind-{}-notes.txt", std::process::id()));
        std::fs::write(&path, "Crosswind thumbnail test\n".repeat(40)).unwrap();
        let data_url = data_url_for_file(&path);
        std::fs::remove_file(&path).unwrap();
        assert!(data_url.is_some_and(|url| url.starts_with("data:image/png;base64,")));
    }
}
