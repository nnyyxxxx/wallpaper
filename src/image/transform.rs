use image::{DynamicImage, GenericImageView};

#[derive(Debug, Clone, Copy)]
pub enum ScalingMode {
    Fill,
    Fit,
    Stretch,
}

impl ScalingMode {
    pub fn scale(
        &self,
        image: &DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> DynamicImage {
        match self {
            ScalingMode::Stretch => image.resize_exact(
                target_width,
                target_height,
                image::imageops::FilterType::Lanczos3,
            ),
            ScalingMode::Fit => {
                let (width, height) = image.dimensions();
                let ratio =
                    (target_width as f64 / width as f64).min(target_height as f64 / height as f64);

                let new_width = (width as f64 * ratio) as u32;
                let new_height = (height as f64 * ratio) as u32;

                image.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
            }
            ScalingMode::Fill => {
                let (width, height) = image.dimensions();
                let ratio =
                    (target_width as f64 / width as f64).max(target_height as f64 / height as f64);

                let new_width = (width as f64 * ratio) as u32;
                let new_height = (height as f64 * ratio) as u32;

                image.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
            }
        }
    }
}
