use image::ImageError;
use thiserror::Error;
use wayland_client::{ConnectError, DispatchError};

#[derive(Error, Debug)]
pub enum WallpaperError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Wayland error: {0}")]
    Wayland(#[from] DispatchError),

    #[error("Wayland connect error: {0}")]
    WaylandConnect(#[from] ConnectError),

    #[error("Image error: {0}")]
    Image(#[from] ImageError),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] Box<bincode::ErrorKind>),

    #[error("Invalid monitor: {0}")]
    InvalidMonitor(String),

    #[error("Invalid scaling mode: {0}")]
    InvalidScaling(String),

    #[error("Wayland protocol error: {0}")]
    WaylandProtocol(String),
}

impl From<memfd::Error> for WallpaperError {
    fn from(err: memfd::Error) -> Self {
        Self::Memory(err.to_string())
    }
}

pub type WallpaperResult<T> = Result<T, WallpaperError>;
