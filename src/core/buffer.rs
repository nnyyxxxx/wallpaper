use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};
use wayland_client::protocol::wl_buffer;

#[derive(Clone)]
pub struct Buffer {
    buffer: wl_buffer::WlBuffer,
    width: u32,
    height: u32,
    released: Arc<AtomicBool>,
    release_count: Arc<AtomicU32>,
}

impl Buffer {
    pub fn new(width: u32, height: u32, buffer: wl_buffer::WlBuffer) -> Self {
        Self {
            buffer,
            width,
            height,
            released: Arc::new(AtomicBool::new(true)),
            release_count: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn buffer(&self) -> &wl_buffer::WlBuffer {
        &self.buffer
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn set_released(&self, released: bool) {
        self.released.store(released, Ordering::Release);
        if released {
            self.release_count.fetch_add(1, Ordering::AcqRel);
        }
    }

    pub fn is_released(&self) -> bool {
        self.released.load(Ordering::Acquire)
    }

    pub fn release_count(&self) -> u32 {
        self.release_count.load(Ordering::Acquire)
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
