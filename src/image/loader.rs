use crate::WallpaperResult;
use image::{imageops::FilterType, DynamicImage};
use log::debug;
use std::time::Instant;

pub struct ImageLoader;

impl ImageLoader {
    pub fn load_and_scale(path: &str, width: i32, height: i32) -> WallpaperResult<DynamicImage> {
        let start = Instant::now();
        debug!("Starting image load for {}x{}", width, height);

        let img = image::open(path)?;
        debug!("Image loaded in {:?}", start.elapsed());

        let scale_start = Instant::now();
        let scaled = if img.width() > width as u32 * 2 || img.height() > height as u32 * 2 {
            img.resize(width as u32, height as u32, FilterType::Triangle)
        } else {
            img.resize_exact(width as u32, height as u32, FilterType::Triangle)
        };
        debug!("Image scaled in {:?}", scale_start.elapsed());

        Ok(scaled)
    }
}
