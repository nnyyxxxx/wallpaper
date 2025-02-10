use crate::{WallpaperError, WallpaperResult};
use cairo::{Context, Format, ImageSurface};
use image::{DynamicImage, GenericImageView, RgbaImage};
use rayon::prelude::*;

pub struct ImageLoader;

impl ImageLoader {
    pub fn load_and_scale(path: &str, width: i32, height: i32) -> WallpaperResult<DynamicImage> {
        let img = image::open(path)?;
        let (img_width, img_height) = img.dimensions();
        let rgba = img.to_rgba8();

        let chunks_size = rgba.len() / rayon::current_num_threads();
        let mut pixel_data = vec![0u8; (width * height * 4) as usize];

        let surface = unsafe {
            ImageSurface::create_for_data_unsafe(
                pixel_data.as_mut_ptr(),
                Format::ARgb32,
                width,
                height,
                width * 4,
            )
        }
        .map_err(|e| WallpaperError::Memory(e.to_string()))?;

        let ctx = Context::new(&surface).map_err(|e| WallpaperError::Memory(e.to_string()))?;

        let scale_x = width as f64 / img_width as f64;
        let scale_y = height as f64 / img_height as f64;
        let scale = scale_x.max(scale_y);

        ctx.scale(scale, scale);

        let bgra: Vec<u8> = rgba
            .par_chunks(chunks_size)
            .flat_map(|chunk| {
                chunk
                    .par_chunks_exact(4)
                    .map(|p| [p[2], p[1], p[0], p[3]])
                    .flatten()
                    .collect::<Vec<_>>()
            })
            .collect();

        let source_surface = ImageSurface::create_for_data(
            bgra,
            Format::ARgb32,
            img_width as i32,
            img_height as i32,
            img_width as i32 * 4,
        )
        .map_err(|e| WallpaperError::Memory(e.to_string()))?;

        ctx.set_source_surface(&source_surface, 0.0, 0.0)
            .map_err(|e| WallpaperError::Memory(e.to_string()))?;

        ctx.paint()
            .map_err(|e| WallpaperError::Memory(e.to_string()))?;

        drop(ctx);
        drop(surface);

        Ok(DynamicImage::ImageRgba8(
            RgbaImage::from_raw(width as u32, height as u32, pixel_data).ok_or_else(|| {
                WallpaperError::Memory("Failed to create image from raw pixels".into())
            })?,
        ))
    }
}
