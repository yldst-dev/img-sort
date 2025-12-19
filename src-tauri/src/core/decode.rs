use anyhow::{anyhow, Result};
use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::DynamicImage;
use std::path::Path;
use std::process::Command;
use tempfile::Builder;

const DEFAULT_MAX_EDGE: u32 = 1280;
const DEFAULT_JPEG_QUALITY: u8 = 75;

pub struct EncodedImage {
    pub base64_jpeg: String,
}

#[derive(Debug, Clone, Copy)]
pub struct DecodeOptions {
    pub resize_enabled: bool,
    pub max_edge: u32,
    pub jpeg_quality: u8,
    pub resize_filter: FilterType,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            resize_enabled: true,
            max_edge: DEFAULT_MAX_EDGE,
            jpeg_quality: DEFAULT_JPEG_QUALITY,
            resize_filter: FilterType::Lanczos3,
        }
    }
}

pub fn decode_resize_base64_with_options(path: &Path, opts: DecodeOptions) -> Result<EncodedImage> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let img = decode_dynamic_image_inner(path, &ext)?;
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let long_edge = w.max(h);
    let jpeg_quality = opts.jpeg_quality.clamp(1, 100);

    let (new_w, new_h, resized) =
        if opts.resize_enabled && opts.max_edge > 0 && long_edge > opts.max_edge {
            let scale = opts.max_edge as f32 / long_edge as f32;
            let new_w = ((w as f32) * scale).round().max(1.0) as u32;
            let new_h = ((h as f32) * scale).round().max(1.0) as u32;
            let resized = image::imageops::resize(&rgb, new_w, new_h, opts.resize_filter);
            (new_w, new_h, resized)
        } else {
            (w, h, rgb)
        };

    let mut buf: Vec<u8> = Vec::new();
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut buf, jpeg_quality);
        encoder.encode(
            resized.as_raw(),
            new_w,
            new_h,
            image::ColorType::Rgb8.into(),
        )?;
    }
    let base64_jpeg = base64::engine::general_purpose::STANDARD.encode(buf);
    Ok(EncodedImage { base64_jpeg })
}

pub fn decode_dynamic_image(path: &Path) -> Result<DynamicImage> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    decode_dynamic_image_inner(path, &ext)
}

fn decode_dynamic_image_inner(path: &Path, ext: &str) -> Result<DynamicImage> {
    match ext {
        "heic" => decode_heic(path),
        "dng" => decode_dng(path),
        _ => Ok(image::open(path)?),
    }
}

fn decode_heic(path: &Path) -> Result<DynamicImage> {
    // macOS: leverage `sips` for HEIC -> JPEG conversion to temp file
    #[cfg(target_os = "macos")]
    {
        let tmp = Builder::new().suffix(".jpg").tempfile()?;
        let out_path = tmp.path().to_owned();
        let status = Command::new("sips")
            .args(["-s", "format", "jpeg", path.to_str().unwrap(), "--out"])
            .arg(&out_path)
            .status()?;
        if !status.success() {
            return Err(anyhow!("sips failed to convert HEIC"));
        }
        let img = image::open(&out_path)?;
        return Ok(img);
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(anyhow!("HEIC decoding not supported on this platform"))
    }
}

fn decode_dng(path: &Path) -> Result<DynamicImage> {
    // Attempt with image crate (tiff/dng) first
    match image::open(path) {
        Ok(img) => Ok(img),
        Err(_) => {
            #[cfg(target_os = "macos")]
            {
                // fallback to sips
                let tmp = Builder::new().suffix(".jpg").tempfile()?;
                let out_path = tmp.path().to_owned();
                let status = Command::new("sips")
                    .args(["-s", "format", "jpeg", path.to_str().unwrap(), "--out"])
                    .arg(&out_path)
                    .status()?;
                if !status.success() {
                    return Err(anyhow!("sips failed to convert DNG"));
                }
                let img = image::open(&out_path)?;
                Ok(img)
            }
            #[cfg(not(target_os = "macos"))]
            {
                Err(anyhow!("DNG decoding not supported on this platform"))
            }
        }
    }
}
