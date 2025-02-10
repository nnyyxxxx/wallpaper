use crate::{WallpaperError, WallpaperResult};
use dashmap::DashMap;
use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageBuffer, Rgba};
use log::debug;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::{
    fs::File,
    io::{BufReader, Read},
    sync::Arc,
    time::Instant,
};
use turbojpeg::{Decompressor, PixelFormat};

static IMAGE_CACHE: Lazy<DashMap<String, Arc<DynamicImage>>> = Lazy::new(DashMap::new);
static DECOMPRESSOR: Lazy<Mutex<Decompressor>> =
    Lazy::new(|| Mutex::new(Decompressor::new().expect("Failed to create JPEG decompressor")));

pub struct ImageLoader;

impl ImageLoader {
    pub fn preload(path: &str) -> WallpaperResult<Arc<DynamicImage>> {
        if let Some(cached) = IMAGE_CACHE.get(path) {
            return Ok(cached.clone());
        }

        let start = Instant::now();

        let mut file = BufReader::with_capacity(1024 * 1024, File::open(path)?);
        let mut jpeg_data = Vec::with_capacity(1024 * 1024);
        file.read_to_end(&mut jpeg_data)?;

        let mut decompressor = DECOMPRESSOR.lock();
        let header = decompressor
            .read_header(&jpeg_data)
            .map_err(|e| WallpaperError::Memory(e.to_string()))?;

        let width = header.width;
        let height = header.height;

        let mut output = vec![0u8; width * height * 4];
        decompressor
            .decompress(
                &jpeg_data,
                turbojpeg::Image {
                    pixels: &mut output,
                    width,
                    height,
                    format: PixelFormat::RGBA,
                    pitch: width * 4,
                },
            )
            .map_err(|e| WallpaperError::Memory(e.to_string()))?;

        let img = ImageBuffer::from_raw(width as u32, height as u32, output)
            .map(DynamicImage::ImageRgba8)
            .ok_or_else(|| WallpaperError::Memory("Failed to create image buffer".into()))?;

        let img = Arc::new(img);
        IMAGE_CACHE.insert(path.to_string(), img.clone());
        debug!("Image loaded in {:?}", start.elapsed());
        Ok(img)
    }

    pub fn scale_image(
        img: &DynamicImage,
        width: u32,
        height: u32,
    ) -> WallpaperResult<DynamicImage> {
        let start = Instant::now();
        let (img_width, img_height) = img.dimensions();

        if img_width == width && img_height == height {
            return Ok(img.clone());
        }

        let scaled = if img_width > width || img_height > height {
            let mut target = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);
            let source = img.to_rgba8();
            let src_pixels = source.as_raw();

            let x_ratio = (img_width << 16) / width;
            let y_ratio = (img_height << 16) / height;

            target.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
                let x = ((i as u32 % width) * x_ratio) >> 16;
                let y = ((i as u32 / width) * y_ratio) >> 16;
                let src_idx = ((y * img_width + x) * 4) as usize;

                chunk.copy_from_slice(&src_pixels[src_idx..src_idx + 4]);
            });

            DynamicImage::ImageRgba8(target)
        } else {
            img.resize_exact(width, height, FilterType::CatmullRom)
        };

        debug!("Total scaling completed in {:?}", start.elapsed());
        Ok(scaled)
    }
}
