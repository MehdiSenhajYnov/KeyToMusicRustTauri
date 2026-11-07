//! Turning a page image into what the model reads: JPEG tiles small enough
//! to be cheap, big enough to read the bubbles.
//!
//! - a manga page: one tile, at most 900 px wide;
//! - a double-page spread: one tile, up to 1600 px wide;
//! - a webtoon strip (much taller than wide): cut into tiles 1400 px tall,
//!   so its text is not shrunk away. Very long strips keep a few tiles
//!   spread along their length.

use std::sync::Arc;

use base64::Engine as _;
use image::{DynamicImage, GenericImageView};

/// One page as sent to the model: base64 JPEG tiles, top to bottom.
pub type PageImage = Arc<Vec<String>>;

const PAGE_WIDTH: u32 = 900;
const SPREAD_WIDTH: u32 = 1600;
const SPREAD_HEIGHT: u32 = 1200;
/// A portrait page up to this tall stays one tile.
const MAX_SINGLE_HEIGHT: u32 = 2400;
const TILE_HEIGHT: u32 = 1400;
const MAX_TILES: usize = 6;
const QUALITY: u8 = 82;

pub fn page(tiles: Vec<String>) -> PageImage {
    Arc::new(tiles)
}

/// From base64 (any common format, `data:` prefix allowed).
pub fn prepare_base64(data: &str) -> Result<Vec<String>, String> {
    let b64 = match data.split_once(";base64,") {
        Some((_, rest)) => rest,
        None => data,
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| format!("base64 invalide : {e}"))?;
    prepare_bytes(&bytes)
}

pub fn prepare_bytes(bytes: &[u8]) -> Result<Vec<String>, String> {
    let image = image::load_from_memory(bytes).map_err(|e| format!("image illisible : {e}"))?;
    prepare(image)
}

fn resize(image: &DynamicImage, scale: f32) -> DynamicImage {
    if scale >= 1.0 {
        return image.clone();
    }
    image.resize_exact(
        ((image.width() as f32 * scale).round() as u32).max(1),
        ((image.height() as f32 * scale).round() as u32).max(1),
        image::imageops::FilterType::Triangle,
    )
}

fn encode(image: &DynamicImage) -> Result<String, String> {
    let mut jpeg = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, QUALITY);
    image
        .to_rgb8()
        .write_with_encoder(encoder)
        .map_err(|e| format!("encodage JPEG impossible : {e}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(jpeg))
}

pub fn prepare(image: DynamicImage) -> Result<Vec<String>, String> {
    let (width, height) = image.dimensions();
    if width < 32 || height < 32 {
        return Err("image trop petite".into());
    }
    if width > height {
        let scale = (SPREAD_WIDTH as f32 / width as f32).min(SPREAD_HEIGHT as f32 / height as f32);
        return Ok(vec![encode(&resize(&image, scale))?]);
    }
    let page = resize(&image, PAGE_WIDTH as f32 / width as f32);
    if page.height() <= MAX_SINGLE_HEIGHT {
        return Ok(vec![encode(&page)?]);
    }
    let count = page.height().div_ceil(TILE_HEIGHT) as usize;
    // Long strips: tiles spread evenly from top to bottom.
    let picked: Vec<usize> = if count <= MAX_TILES {
        (0..count).collect()
    } else {
        (0..MAX_TILES)
            .map(|i| i * (count - 1) / (MAX_TILES - 1))
            .collect()
    };
    picked
        .into_iter()
        .map(|i| {
            let top = i as u32 * TILE_HEIGHT;
            let tile_height = TILE_HEIGHT.min(page.height() - top);
            encode(&page.crop_imm(0, top, page.width(), tile_height))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbImage::from_pixel(width, height, image::Rgb([200, 200, 200]));
        let mut bytes = Vec::new();
        DynamicImage::ImageRgb8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        bytes
    }

    fn sizes(tiles: &[String]) -> Vec<(u32, u32)> {
        tiles
            .iter()
            .map(|t| {
                let bytes = base64::engine::general_purpose::STANDARD.decode(t).unwrap();
                image::load_from_memory(&bytes).unwrap().dimensions()
            })
            .collect()
    }

    #[test]
    fn pages_spreads_and_strips() {
        assert_eq!(
            sizes(&prepare_bytes(&png(1800, 2600)).unwrap()),
            [(900, 1300)]
        );
        assert_eq!(sizes(&prepare_bytes(&png(600, 900)).unwrap()), [(600, 900)]);
        assert_eq!(
            sizes(&prepare_bytes(&png(3600, 2600)).unwrap()),
            [(1600, 1156)]
        );
        // Never enlarged: 800×3000 makes three tiles.
        assert_eq!(
            sizes(&prepare_bytes(&png(800, 3000)).unwrap()),
            [(800, 1400), (800, 1400), (800, 200)]
        );
        // Very long strip: six tiles, the last one at the bottom.
        let tiles = sizes(&prepare_bytes(&png(800, 20000)).unwrap());
        assert_eq!(tiles.len(), MAX_TILES);
        assert!(tiles.last().unwrap().1 < 1400);
    }

    #[test]
    fn base64_with_prefix_and_garbage() {
        let data = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png(400, 600))
        );
        assert_eq!(prepare_base64(&data).unwrap().len(), 1);
        assert!(prepare_base64("not base64!").is_err());
        assert!(prepare_bytes(&png(20, 20)).is_err());
    }
}
