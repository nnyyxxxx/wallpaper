use crate::{
    core::{
        backend::LayerSurface,
        buffer::Buffer,
        cache::{Cache, CacheKey},
        pool::BumpPool,
    },
    display::monitor::Monitor,
    image::loader::ImageLoader,
    utils::{
        error::{WallpaperError, WallpaperResult},
        wayland::WaylandState,
    },
};
use log::{debug, info};
use rayon::prelude::*;
use std::cell::RefCell;
use wayland_client::{Connection, EventQueue, Proxy};

pub struct App {
    monitors: Vec<Monitor>,
    current_wallpaper: RefCell<Option<String>>,
    wayland_state: Option<WaylandState>,
    connection: Option<Connection>,
    surfaces: Vec<LayerSurface>,
    event_queue: Option<EventQueue<WaylandState>>,
    cache: Cache,
}

impl App {
    pub fn new() -> WallpaperResult<Self> {
        let mut app = Self {
            monitors: Vec::new(),
            current_wallpaper: RefCell::new(None),
            wayland_state: None,
            connection: None,
            surfaces: Vec::new(),
            event_queue: None,
            cache: Cache::new(),
        };
        app.init_wayland()?;
        Ok(app)
    }

    pub fn event_queue(&self) -> &EventQueue<WaylandState> {
        self.event_queue
            .as_ref()
            .expect("Event queue should be initialized")
    }

    pub fn state(&self) -> &WaylandState {
        self.wayland_state
            .as_ref()
            .expect("Wayland state should be initialized")
    }

    pub fn init_wayland(&mut self) -> WallpaperResult<()> {
        let conn = Connection::connect_to_env()?;
        let mut event_queue = conn.new_event_queue();
        let qh = event_queue.handle();

        let mut state = WaylandState::new(&conn, &qh)?;
        event_queue.roundtrip(&mut state)?;
        event_queue.roundtrip(&mut state)?;

        self.monitors = state.get_monitors().to_vec();
        self.connection = Some(conn);
        self.wayland_state = Some(state);
        self.event_queue = Some(event_queue);
        self.recreate_surfaces()?;

        Ok(())
    }

    pub fn set_wallpaper(
        &mut self,
        path: &str,
        event_queue: &EventQueue<WaylandState>,
        state: &WaylandState,
    ) -> WallpaperResult<()> {
        info!("Setting wallpaper: {}", path);
        let path = path.to_string();
        let qh = &event_queue.handle();
        let cache = &self.cache;

        let buffers: Vec<_> = self
            .monitors
            .par_iter()
            .map(|monitor| {
                let cache_key = CacheKey::new(
                    &path,
                    monitor.width.try_into().unwrap(),
                    monitor.height.try_into().unwrap(),
                );
                if let Some(cached_buffer) = cache.get(&cache_key) {
                    return Ok(cached_buffer.clone());
                }

                let scaled = ImageLoader::load_and_scale(&path, monitor.width, monitor.height)?;
                let mut pool = BumpPool::new(monitor.width, monitor.height)?;
                let rgba = scaled.to_rgba8();
                pool.write_pixels(rgba.as_raw());
                let buffer = pool.get_buffer(state.get_shm(), qh).clone();
                Ok::<Buffer, WallpaperError>(buffer)
            })
            .collect::<Result<_, _>>()?;

        for (surface, buffer) in self.surfaces.iter_mut().zip(buffers.iter()) {
            surface.attach_buffer(buffer, qh);
        }

        for (monitor, buffer) in self.monitors.iter().zip(buffers.iter()) {
            let cache_key = CacheKey::new(
                &path,
                monitor.width.try_into().unwrap(),
                monitor.height.try_into().unwrap(),
            );
            self.cache.insert(cache_key, buffer.clone());
        }

        self.current_wallpaper = RefCell::new(Some(path));
        Ok(())
    }

    pub fn run_event_loop(&mut self) -> WallpaperResult<()> {
        info!("Starting event loop");
        let mut event_queue = self
            .event_queue
            .take()
            .expect("Event queue should be initialized");
        let mut state = self
            .wayland_state
            .take()
            .expect("Wayland state should be initialized");

        debug!("Initial roundtrip");
        event_queue.roundtrip(&mut state)?;
        event_queue.roundtrip(&mut state)?;

        let current_path = self.current_wallpaper.borrow().clone();

        if let Some(path) = current_path {
            debug!("Setting initial wallpaper");
            let qh = &event_queue.handle();

            let buffers: Vec<_> = self
                .monitors
                .iter()
                .map(|monitor| {
                    let scaled = ImageLoader::load_and_scale(&path, monitor.width, monitor.height)?;
                    let mut pool = BumpPool::new(monitor.width, monitor.height)?;
                    let rgba = scaled.to_rgba8();
                    pool.write_pixels(rgba.as_raw());
                    Ok::<Buffer, WallpaperError>(pool.get_buffer(state.get_shm(), qh).clone())
                })
                .collect::<Result<_, _>>()?;

            for (surface, buffer) in self.surfaces.iter_mut().zip(buffers.iter()) {
                surface.attach_buffer(buffer, qh);
            }
        }

        info!("Entering main event loop");
        loop {
            event_queue.blocking_dispatch(&mut state)?;
        }
    }

    fn recreate_surfaces(&mut self) -> WallpaperResult<()> {
        self.surfaces.clear();

        let state = self
            .wayland_state
            .as_mut()
            .expect("Wayland state should be initialized");
        let mut event_queue = self
            .event_queue
            .take()
            .expect("Event queue should be initialized");
        let qh = event_queue.handle();
        let conn = self
            .connection
            .as_ref()
            .expect("Connection should be initialized");

        for monitor in &self.monitors {
            let surface = LayerSurface::new(
                conn,
                &qh,
                monitor,
                state.get_layer_shell(),
                state.get_compositor(),
                Some(state.get_viewporter()),
            )?;

            debug!("Created layer surface with id: {:?}", surface.layer().id());
            state.add_layer_surface(surface.layer().id().protocol_id(), surface.clone());
            self.surfaces.push(surface);
        }

        event_queue.roundtrip(state)?;

        for _ in 0..10 {
            event_queue.roundtrip(state)?;
            if state.all_surfaces_configured() {
                debug!("All surfaces configured");
                break;
            }
        }

        self.event_queue = Some(event_queue);
        Ok(())
    }

    pub fn set_wallpaper_and_exit(&mut self, path: &str) -> WallpaperResult<()> {
        info!("Setting wallpaper: {}", path);

        let mut event_queue = self
            .event_queue
            .take()
            .expect("Event queue should be initialized");
        let mut state = self
            .wayland_state
            .take()
            .expect("Wayland state should be initialized");

        event_queue.roundtrip(&mut state)?;
        event_queue.roundtrip(&mut state)?;

        let qh = event_queue.handle();
        let buffers: Vec<_> = self
            .monitors
            .iter()
            .map(|monitor| {
                let cache_key = CacheKey::new(
                    path,
                    monitor.width.try_into().unwrap(),
                    monitor.height.try_into().unwrap(),
                );
                if let Some(cached_buffer) = self.cache.get(&cache_key) {
                    return Ok(cached_buffer.clone());
                }

                let scaled = ImageLoader::load_and_scale(path, monitor.width, monitor.height)?;
                let mut pool = BumpPool::new(monitor.width, monitor.height)?;
                let rgba = scaled.to_rgba8();
                pool.write_pixels(rgba.as_raw());
                let buffer = pool.get_buffer(state.get_shm(), &qh).clone();
                self.cache.insert(cache_key, buffer.clone());
                Ok::<Buffer, WallpaperError>(buffer)
            })
            .collect::<Result<_, _>>()?;

        for (surface, buffer) in self.surfaces.iter_mut().zip(buffers.iter()) {
            surface.attach_buffer(buffer, &qh);
        }

        self.current_wallpaper = RefCell::new(Some(path.to_string()));

        event_queue.roundtrip(&mut state)?;
        event_queue.roundtrip(&mut state)?;

        self.event_queue = Some(event_queue);
        self.wayland_state = Some(state);
        Ok(())
    }
}
