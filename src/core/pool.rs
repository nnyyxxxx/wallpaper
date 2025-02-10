use crate::{
    core::buffer::Buffer,
    utils::{error::WallpaperResult, wayland::WaylandState},
};
use log::debug;
use memmap2::{MmapMut, MmapOptions};
use std::os::fd::{AsRawFd, BorrowedFd};
use wayland_client::{
    protocol::{wl_shm, wl_shm_pool::WlShmPool},
    Proxy, QueueHandle,
};

const MIN_POOL_SIZE: usize = 4096;
const BUFFER_COUNT: usize = 2;

pub struct BufferPool {
    mmap: MmapMut,
    fd: memfd::Memfd,
    size: usize,
    width: i32,
    height: i32,
    current_index: usize,
    buffers: Vec<Buffer>,
    pool: Option<WlShmPool>,
}

impl BufferPool {
    pub fn new(width: i32, height: i32) -> WallpaperResult<Self> {
        let min_size = (width * height * 4) as usize * BUFFER_COUNT;
        let size = min_size.max(MIN_POOL_SIZE);

        let fd = memfd::MemfdOptions::new()
            .allow_sealing(true)
            .close_on_exec(true)
            .create("wallpaper")?;

        fd.as_file().set_len(size as u64)?;
        let mmap = unsafe { MmapOptions::new().len(size).map_mut(&fd)? };

        Ok(Self {
            mmap,
            fd,
            size,
            width,
            height,
            current_index: 0,
            buffers: Vec::with_capacity(BUFFER_COUNT),
            pool: None,
        })
    }

    pub fn write_pixels(&mut self, pixels: &[u8]) {
        debug!(
            "Writing pixels to buffer pool at index {}",
            self.current_index
        );
        let start = self.current_index * (self.width * self.height * 4) as usize;
        unsafe {
            std::ptr::copy_nonoverlapping(
                pixels.as_ptr(),
                self.mmap[start..].as_mut_ptr(),
                pixels.len(),
            );
        }
        debug!("Pixel data written successfully");
    }

    pub fn get_buffer(&mut self, shm: &wl_shm::WlShm, qh: &QueueHandle<WaylandState>) -> &Buffer {
        debug!(
            "Getting buffer from pool (current index: {})",
            self.current_index
        );
        if self.buffers.len() < BUFFER_COUNT {
            debug!("Creating new buffer in pool");
            let offset = self.current_index * (self.width * self.height * 4) as usize;
            let buffer = self.create_buffer(shm, qh, offset);
            self.buffers.push(buffer);
        }

        let buffer = &self.buffers[self.current_index];
        debug!("Returning buffer {:?}", buffer.buffer().id());
        self.current_index = (self.current_index + 1) % BUFFER_COUNT;
        buffer
    }

    fn create_buffer(
        &mut self,
        shm: &wl_shm::WlShm,
        qh: &QueueHandle<WaylandState>,
        offset: usize,
    ) -> Buffer {
        if self.pool.is_none() {
            self.pool = Some(shm.create_pool(
                unsafe { BorrowedFd::borrow_raw(self.fd.as_raw_fd()) },
                self.size as i32,
                qh,
                (),
            ));
        }

        let pool = self.pool.as_ref().unwrap();
        let stride = self.width * 4;
        let buffer = pool.create_buffer(
            offset as i32,
            self.width,
            self.height,
            stride,
            wl_shm::Format::Xrgb8888,
            qh,
            (),
        );

        Buffer::new(
            self.width.try_into().unwrap(),
            self.height.try_into().unwrap(),
            buffer,
        )
    }
}

impl Drop for BufferPool {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.take() {
            pool.destroy();
        }
    }
}
