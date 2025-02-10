use crate::{
    core::buffer::Buffer,
    utils::{error::WallpaperResult, wayland::WaylandState},
};
use memmap2::{MmapMut, MmapOptions};
use std::{
    io::Write,
    os::fd::{AsRawFd, BorrowedFd},
};
use wayland_client::{protocol::wl_shm, QueueHandle};

pub struct BumpPool {
    mmap: MmapMut,
    width: i32,
    height: i32,
    last_used_buffer: usize,
    buffers: Vec<Buffer>,
}

impl BumpPool {
    pub fn new(width: i32, height: i32) -> WallpaperResult<Self> {
        let size = (width * height * 4) as usize;
        let fd = memfd::MemfdOptions::new()
            .close_on_exec(true)
            .create("wallpaper")?;

        fd.as_file().set_len(size as u64)?;

        let mmap = unsafe { MmapOptions::new().len(size).map_mut(&fd)? };

        Ok(Self {
            mmap,
            width,
            height,
            last_used_buffer: 0,
            buffers: Vec::new(),
        })
    }

    pub fn get_buffer(&mut self, shm: &wl_shm::WlShm, qh: &QueueHandle<WaylandState>) -> &Buffer {
        if let Some((idx, _)) = self
            .buffers
            .iter()
            .enumerate()
            .find(|(_, b)| b.is_released())
        {
            self.last_used_buffer = idx;
            return &self.buffers[idx];
        }

        let buffer = self.create_buffer(shm, qh);
        self.buffers.push(buffer);
        self.last_used_buffer = self.buffers.len() - 1;
        &self.buffers[self.last_used_buffer]
    }

    fn create_buffer(&self, shm: &wl_shm::WlShm, qh: &QueueHandle<WaylandState>) -> Buffer {
        let fd = memfd::MemfdOptions::new()
            .close_on_exec(true)
            .create("wallpaper")
            .expect("Failed to create memfd");

        let size = (self.width * self.height * 4) as u64;
        fd.as_file()
            .set_len(size)
            .expect("Failed to set memfd size");

        fd.as_file()
            .write_all(&self.mmap)
            .expect("Failed to write buffer data");

        let pool = shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(fd.as_raw_fd()) },
            self.width * self.height * 4,
            qh,
            (),
        );

        let buffer = pool.create_buffer(
            0,
            self.width,
            self.height,
            self.width * 4,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );

        pool.destroy();
        Buffer::new(self.width as u32, self.height as u32, buffer)
    }

    pub fn write_pixels(&mut self, pixels: &[u8]) {
        for chunk in pixels.chunks_exact(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            let idx = chunk.as_ptr() as usize - pixels.as_ptr() as usize;
            self.mmap[idx..idx + 4].copy_from_slice(&[b, g, r, a]);
        }
    }
}
