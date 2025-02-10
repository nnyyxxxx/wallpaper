pub mod core {
    pub mod app;
    pub mod backend;
    pub mod buffer;
    pub mod cache;
    pub mod daemon;
    pub mod ipc;
    pub mod pool;
    pub mod shm;
}

pub mod utils {
    pub mod cli;
    pub mod error;
    pub mod wayland;
}

pub mod display {
    pub mod monitor;
}

pub mod image {
    pub mod loader;
}

pub use core::{app::App, daemon::Daemon};
pub use utils::error::{WallpaperError, WallpaperResult};
