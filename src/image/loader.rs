use crate::{WallpaperError, WallpaperResult};
use fast_image_resize::{Image as ResizeImage, PixelType, ResizeAlg, Resizer};
use image::{DynamicImage, GenericImageView};
use rayon::prelude::*;
use std::num::NonZeroU32;

pub struct ImageLoader;

impl ImageLoader {
    pub fn load_and_scale(path: &str, width: i32, height: i32) -> WallpaperResult<DynamicImage> {
        let img = image::open(path)?;
        let (img_width, img_height) = img.dimensions();

        let rgba = img.to_rgba8();
        let src = ResizeImage::from_vec_u8(
            NonZeroU32::new(img_width).unwrap(),
            NonZeroU32::new(img_height).unwrap(),
            rgba.into_raw(),
            PixelType::U8x4,
        )
        .unwrap();

        let mut dst = ResizeImage::new(
            NonZeroU32::new(width as u32).unwrap(),
            NonZeroU32::new(height as u32).unwrap(),
            PixelType::U8x4,
        );

        let mut resizer = Resizer::new(ResizeAlg::Convolution(
            fast_image_resize::FilterType::Lanczos3,
        ));
        resizer
            .resize(&src.view(), &mut dst.view_mut())
            .map_err(|e| WallpaperError::Memory(e.to_string()))?;

        let mut pixels = dst.into_vec();

        pixels.par_chunks_exact_mut(4).for_each(|chunk| {
            let r = chunk[0];
            let b = chunk[2];
            chunk[0] = b;
            chunk[2] = r;
        });

        Ok(DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(width as u32, height as u32, pixels).unwrap(),
        ))
    }
}
