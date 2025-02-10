use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use wayland_client::protocol::wl_buffer;

#[derive(Clone)]
pub struct Buffer {
    buffer: wl_buffer::WlBuffer,
    width: u32,
    height: u32,
    released: Arc<AtomicBool>,
}

impl Buffer {
    pub fn new(width: u32, height: u32, buffer: wl_buffer::WlBuffer) -> Self {
        Self {
            buffer,
            width,
            height,
            released: Arc::new(AtomicBool::new(true)),
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
        self.released.store(released, Ordering::SeqCst);
    }

    pub fn is_released(&self) -> bool {
        self.released.load(Ordering::SeqCst)
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
