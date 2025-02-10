use crate::{
    core::buffer::Buffer,
    utils::{
        error::{WallpaperError, WallpaperResult},
        wayland::WaylandState,
    },
};
use memmap2::{MmapMut, MmapOptions};
use std::{
    io::Write,
    os::{fd::BorrowedFd, unix::io::AsRawFd},
};
use wayland_client::{protocol::wl_shm, QueueHandle};

pub struct ShmBuffer {
    mmap: MmapMut,
    size: (u32, u32),
}

impl ShmBuffer {
    pub fn new(width: u32, height: u32) -> WallpaperResult<Self> {
        let stride = width * 4;
        let size = stride * height;

        let fd = memfd::MemfdOptions::new()
            .close_on_exec(true)
            .create("wallpaper")
            .map_err(|e| {
                WallpaperError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        fd.as_file()
            .set_len(size as u64)
            .map_err(WallpaperError::IoError)?;

        let mmap = unsafe {
            MmapOptions::new()
                .len(size as usize)
                .map_mut(&fd)
                .map_err(WallpaperError::IoError)?
        };

        Ok(Self {
            mmap,
            size: (width, height),
        })
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.mmap[..]
    }

    pub fn create_buffer(&self, shm: &wl_shm::WlShm, qh: &QueueHandle<WaylandState>) -> Buffer {
        let fd = memfd::MemfdOptions::new()
            .close_on_exec(true)
            .create("wallpaper")
            .expect("Failed to create memfd");

        let size = (self.size.0 * self.size.1 * 4) as u64;
        fd.as_file()
            .set_len(size)
            .expect("Failed to set memfd size");

        fd.as_file()
            .write_all(&self.mmap)
            .expect("Failed to write buffer data");

        let pool = shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(fd.as_raw_fd()) },
            size as i32,
            qh,
            (),
        );

        let wl_buffer = pool.create_buffer(
            0,
            self.size.0 as i32,
            self.size.1 as i32,
            (self.size.0 * 4) as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );

        pool.destroy();

        Buffer::new(self.size.0, self.size.1, wl_buffer)
    }
}
