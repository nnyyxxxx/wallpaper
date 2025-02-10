use crate::WallpaperResult;
use image::DynamicImage;

pub struct ImageLoader;

impl ImageLoader {
    pub fn load_and_scale(path: &str, width: i32, height: i32) -> WallpaperResult<DynamicImage> {
        let img = image::open(path)?;
        let scaled = img.resize_exact(
            width as u32,
            height as u32,
            image::imageops::FilterType::Lanczos3,
        );
        Ok(scaled)
    }
}
