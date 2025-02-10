use crate::{WallpaperError, WallpaperResult};
use cairo::{Context, Format, ImageSurface};
use image::{DynamicImage, GenericImageView};

const BUFFER_STRIDE_ALIGN: usize = 4;

pub struct ImageLoader;

impl ImageLoader {
    pub fn load_and_scale(path: &str, width: i32, height: i32) -> WallpaperResult<DynamicImage> {
        let img = image::open(path)?;
        let (img_width, img_height) = img.dimensions();

        let stride = ((width * 4) as usize + BUFFER_STRIDE_ALIGN - 1) & !(BUFFER_STRIDE_ALIGN - 1);
        let mut pixel_data = vec![0u8; stride * height as usize];

        let surface = unsafe {
            ImageSurface::create_for_data_unsafe(
                pixel_data.as_mut_ptr(),
                Format::Rgb24,
                width,
                height,
                stride as i32,
            )
        }
        .map_err(|e| WallpaperError::Memory(e.to_string()))?;

        let ctx = Context::new(&surface)?;
        let scale_x = width as f64 / img_width as f64;
        let scale_y = height as f64 / img_height as f64;
        let scale = scale_x.max(scale_y);

        ctx.scale(scale, scale);

        let rgba = img.to_rgba8();
        let source_surface = ImageSurface::create_for_data(
            rgba.as_raw().to_vec(),
            Format::Rgb24,
            img_width as i32,
            img_height as i32,
            (img_width * 4) as i32,
        )?;

        ctx.set_source_surface(&source_surface, 0.0, 0.0)?;
        ctx.paint()?;

        Ok(DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(width as u32, height as u32, pixel_data)
                .ok_or_else(|| WallpaperError::Memory("Failed to create image".into()))?,
        ))
    }
}
