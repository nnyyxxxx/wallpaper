use crate::WallpaperResult;
use image::{imageops::FilterType, DynamicImage};

pub struct ImageLoader;

impl ImageLoader {
    pub fn load_and_scale(path: &str, width: i32, height: i32) -> WallpaperResult<DynamicImage> {
        let img = image::open(path)?;
        let scaled = img.resize(width as u32, height as u32, FilterType::Lanczos3);

        Ok(scaled)
    }
}
