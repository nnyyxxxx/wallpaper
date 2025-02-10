use crate::{
    core::{
        backend::LayerSurface,
        buffer::Buffer,
        cache::{Cache, CacheKey},
        pool::BufferPool,
    },
    display::monitor::Monitor,
    image::loader::ImageLoader,
    utils::{
        error::{WallpaperError, WallpaperResult},
        wayland::WaylandState,
    },
};
use log::{debug, info};
use parking_lot::RwLock;
use rayon::prelude::*;
use std::sync::Arc;
use wayland_client::{Connection, EventQueue, Proxy};

pub struct App {
    monitors: Vec<Monitor>,
    current_wallpaper: RwLock<Option<String>>,
    wayland_state: Option<WaylandState>,
    connection: Option<Connection>,
    surfaces: Vec<LayerSurface>,
    event_queue: Option<EventQueue<WaylandState>>,
    cache: Arc<RwLock<Cache>>,
}

impl App {
    pub fn new() -> WallpaperResult<Self> {
        let mut app = Self {
            monitors: Vec::new(),
            current_wallpaper: RwLock::new(None),
            wayland_state: None,
            connection: None,
            surfaces: Vec::new(),
            event_queue: None,
            cache: Arc::new(RwLock::new(Cache::new())),
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

        let current_path = self.current_wallpaper.read().clone();

        if let Some(path) = current_path {
            debug!("Setting initial wallpaper");
            let qh = &event_queue.handle();

            let img = ImageLoader::preload(&path)?;
            let buffers: Vec<_> = self
                .monitors
                .iter()
                .enumerate()
                .par_bridge()
                .map(|(i, monitor)| {
                    debug!("Creating new buffer for monitor {}", i);
                    let scaled = ImageLoader::scale_image(
                        &img,
                        monitor.width as u32,
                        monitor.height as u32,
                    )?;
                    let mut pool = BufferPool::new(monitor.width, monitor.height)?;
                    pool.write_pixels(scaled.to_rgba8().as_raw());
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

        let new_surfaces = self
            .monitors
            .iter()
            .map(|monitor| {
                LayerSurface::new(
                    conn,
                    &qh,
                    monitor,
                    state.get_layer_shell(),
                    state.get_compositor(),
                    Some(state.get_viewporter()),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        for surface in &new_surfaces {
            debug!("Created layer surface with id: {:?}", surface.layer().id());
            state.add_layer_surface(surface.layer().id().protocol_id(), surface.clone());
        }

        const MAX_CONFIGURE_ATTEMPTS: u8 = 10;
        for _ in 0..MAX_CONFIGURE_ATTEMPTS {
            event_queue.roundtrip(state)?;
            if state.all_surfaces_configured() {
                debug!("All surfaces configured");
                break;
            }
        }

        self.surfaces = new_surfaces;
        self.event_queue = Some(event_queue);
        Ok(())
    }

    pub fn set_wallpaper_and_exit(&mut self, path: &str) -> WallpaperResult<()> {
        info!("Setting wallpaper: {}", path);
        debug!("Starting wallpaper setting process");

        let mut event_queue = self
            .event_queue
            .take()
            .expect("Event queue should be initialized");
        let mut state = self
            .wayland_state
            .take()
            .expect("Wayland state should be initialized");
        let qh = event_queue.handle();
        let cache = Arc::clone(&self.cache);

        debug!("Creating buffers for {} monitors", self.monitors.len());
        let img = ImageLoader::preload(path)?;

        let buffers: Vec<_> = self
            .monitors
            .iter()
            .enumerate()
            .par_bridge()
            .map(|(i, monitor)| {
                let cache_key = CacheKey::new(
                    path,
                    monitor.width.try_into().unwrap(),
                    monitor.height.try_into().unwrap(),
                );

                if let Some(buffer) = cache.read().get(&cache_key) {
                    return Ok(buffer.clone());
                }

                debug!("Creating new buffer for monitor {}", i);
                let scaled =
                    ImageLoader::scale_image(&img, monitor.width as u32, monitor.height as u32)?;
                let mut pool = BufferPool::new(monitor.width, monitor.height)?;
                pool.write_pixels(scaled.to_rgba8().as_raw());
                let buffer = pool.get_buffer(state.get_shm(), &qh).clone();

                cache.write().insert(cache_key, buffer.clone());
                Ok::<Buffer, WallpaperError>(buffer)
            })
            .collect::<Result<_, _>>()?;

        debug!("Attaching buffers to surfaces");
        for (i, (surface, buffer)) in self.surfaces.iter_mut().zip(buffers).enumerate() {
            debug!(
                "Attaching buffer {:?} to surface {}",
                buffer.buffer().id(),
                i
            );
            surface.attach_buffer(&buffer, &qh);
        }

        self.current_wallpaper = RwLock::new(Some(path.to_string()));
        event_queue.roundtrip(&mut state)?;

        self.event_queue = Some(event_queue);
        self.wayland_state = Some(state);
        Ok(())
    }
}
