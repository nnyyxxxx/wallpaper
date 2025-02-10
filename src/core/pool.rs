use crate::{
    core::buffer::Buffer,
    utils::{error::WallpaperResult, wayland::WaylandState},
};
use log::debug;
use memmap2::{MmapMut, MmapOptions};
use std::{
    os::fd::{AsRawFd, BorrowedFd},
    time::Instant,
};
use wayland_client::{
    protocol::{wl_shm, wl_shm_pool::WlShmPool},
    Proxy, QueueHandle,
};

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
    stride: i32,
}

impl BufferPool {
    pub fn new(width: i32, height: i32) -> WallpaperResult<Self> {
        let stride = width * 4;
        let size = (height * stride) as usize;

        debug!("Creating buffer pool with size: {}MB", size / 1024 / 1024);
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
            stride,
        })
    }

    pub fn write_pixels(&mut self, pixels: &[u8]) {
        let start = Instant::now();
        debug!("Starting pixel write of {}MB", pixels.len() / 1024 / 1024);

        let dst = &mut self.mmap[..pixels.len()];
        for i in 0..(pixels.len() / 4) {
            dst[i * 4] = pixels[i * 4 + 2];
            dst[i * 4 + 1] = pixels[i * 4 + 1];
            dst[i * 4 + 2] = pixels[i * 4];
            dst[i * 4 + 3] = 255;
        }

        debug!("Pixel write completed in {:?}", start.elapsed());
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
            let start = Instant::now();
            self.pool = Some(shm.create_pool(
                unsafe { BorrowedFd::borrow_raw(self.fd.as_raw_fd()) },
                self.size as i32,
                qh,
                (),
            ));
            debug!("Pool creation took {:?}", start.elapsed());
        }

        let pool = self.pool.as_ref().unwrap();
        let start = Instant::now();
        let buffer = pool.create_buffer(
            offset as i32,
            self.width,
            self.height,
            self.stride,
            wl_shm::Format::Xrgb8888,
            qh,
            (),
        );
        debug!("Buffer creation took {:?}", start.elapsed());

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
