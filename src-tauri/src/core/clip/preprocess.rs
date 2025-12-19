use crate::core::decode::decode_dynamic_image;
use anyhow::Result;
use image::imageops::FilterType;
use std::path::Path;

const SIZE: u32 = 224;

// CLIP normalization constants (OpenAI CLIP)
const MEAN: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
const STD: [f32; 3] = [0.26862954, 0.26130258, 0.27577711];

pub struct PreprocessOutput {
    pub nchw: Vec<f32>,
}

pub fn preprocess_clip_image(path: &Path) -> Result<PreprocessOutput> {
    let img = decode_dynamic_image(path)?;
    let rgb = img.to_rgb8();
    let resized = image::imageops::resize(&rgb, SIZE, SIZE, FilterType::Triangle);
    let (w, h) = resized.dimensions();

    let mut nchw = vec![0.0f32; (3 * w * h) as usize];
    // NCHW with channel-first
    for y in 0..h {
        for x in 0..w {
            let p = resized.get_pixel(x, y).0;
            let r = (p[0] as f32) / 255.0;
            let g = (p[1] as f32) / 255.0;
            let b = (p[2] as f32) / 255.0;

            let r = (r - MEAN[0]) / STD[0];
            let g = (g - MEAN[1]) / STD[1];
            let b = (b - MEAN[2]) / STD[2];

            let idx = (y * w + x) as usize;
            nchw[idx] = r;
            nchw[(w * h) as usize + idx] = g;
            nchw[(2 * w * h) as usize + idx] = b;
        }
    }

    let _ = (w, h);
    Ok(PreprocessOutput { nchw })
}
