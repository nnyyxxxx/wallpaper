use wayland_client::{
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_output, wl_registry, wl_shm, wl_shm_pool,
        wl_surface,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

use crate::{
    core::backend::LayerSurface, display::monitor::Monitor, utils::error::WallpaperResult,
};
use log::{debug, info};
use std::collections::HashMap;

pub struct WaylandState {
    pub(crate) monitors: Vec<Monitor>,
    pub(crate) outputs: HashMap<u32, wl_output::WlOutput>,
    pub(crate) shm: Option<wl_shm::WlShm>,
    pub(crate) layer_shell: Option<ZwlrLayerShellV1>,
    pub(crate) compositor: Option<wl_compositor::WlCompositor>,
    pub(crate) viewporter: Option<wp_viewporter::WpViewporter>,
    pub(crate) layer_surfaces: HashMap<u32, LayerSurface>,
}

impl WaylandState {
    pub fn new(conn: &Connection, qh: &QueueHandle<Self>) -> WallpaperResult<Self> {
        let state = Self {
            monitors: Vec::new(),
            outputs: HashMap::new(),
            shm: None,
            layer_shell: None,
            compositor: None,
            viewporter: None,
            layer_surfaces: HashMap::new(),
        };

        conn.display().get_registry(qh, ());

        Ok(state)
    }

    pub fn get_monitors(&self) -> &[Monitor] {
        &self.monitors
    }

    pub fn get_shm(&self) -> &wl_shm::WlShm {
        self.shm.as_ref().expect("SHM should be initialized")
    }

    pub fn get_layer_shell(&self) -> &ZwlrLayerShellV1 {
        self.layer_shell
            .as_ref()
            .expect("Layer shell should be initialized")
    }

    pub fn get_compositor(&self) -> &wl_compositor::WlCompositor {
        self.compositor
            .as_ref()
            .expect("Compositor should be initialized")
    }

    pub fn get_viewporter(&self) -> &wp_viewporter::WpViewporter {
        self.viewporter
            .as_ref()
            .expect("Viewporter should be initialized")
    }

    pub fn add_layer_surface(&mut self, id: u32, surface: LayerSurface) {
        debug!("Adding layer surface with id: {}", id);
        self.layer_surfaces.insert(id, surface);
    }

    pub fn get_layer_surface(&mut self, id: u32) -> Option<&mut LayerSurface> {
        self.layer_surfaces.get_mut(&id)
    }

    pub fn all_surfaces_configured(&self) -> bool {
        self.layer_surfaces.values().all(|s| s.is_configured())
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                debug!("Registering interface: {}", interface);
                match interface.as_str() {
                    "wl_output" => {
                        let output =
                            registry.bind::<wl_output::WlOutput, _, _>(name, version, qh, ());
                        state.monitors.push(Monitor::new(output, name));
                        info!("Registered output device");
                    }
                    "wl_compositor" => {
                        let compositor = registry.bind::<wl_compositor::WlCompositor, _, _>(
                            name,
                            version,
                            qh,
                            (),
                        );
                        state.compositor = Some(compositor);
                        info!("Registered compositor");
                    }
                    "zwlr_layer_shell_v1" => {
                        let layer_shell =
                            registry.bind::<ZwlrLayerShellV1, _, _>(name, version, qh, ());
                        state.layer_shell = Some(layer_shell);
                        info!("Registered layer shell");
                    }
                    "wl_shm" => {
                        let shm = registry.bind::<wl_shm::WlShm, _, _>(name, version, qh, ());
                        state.shm = Some(shm);
                        info!("Registered SHM");
                    }
                    "wp_viewporter" => {
                        let viewporter = registry.bind::<wp_viewporter::WpViewporter, _, _>(
                            name,
                            version,
                            qh,
                            (),
                        );
                        state.viewporter = Some(viewporter);
                        info!("Registered viewporter");
                    }
                    _ => {}
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                state.outputs.remove(&name);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Geometry {
                x,
                y,
                physical_width: _,
                physical_height: _,
                subpixel: _,
                make,
                model,
                transform: _,
            } => {
                debug!(
                    "Output geometry: pos=({}, {}), make={}, model={}",
                    x, y, make, model
                );

                if let Some(monitor) = state
                    .monitors
                    .iter_mut()
                    .find(|m| m.output.id() == output.id())
                {
                    monitor.x = x;
                    monitor.y = y;
                }
            }
            wl_output::Event::Mode {
                flags: _,
                width,
                height,
                refresh,
            } => {
                debug!("Output mode: {}x{}", width, height);
                if let Some(monitor) = state
                    .monitors
                    .iter_mut()
                    .find(|m| m.output.id() == output.id())
                {
                    monitor.width = width;
                    monitor.height = height;
                    monitor.refresh = refresh;
                }
            }
            wl_output::Event::Done => {
                debug!("Output configuration done");
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_shm::WlShm, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: <ZwlrLayerShellV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_surface::WlSurface,
        _: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_shm_pool::WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for WaylandState {
    fn event(
        state: &mut Self,
        buffer: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            debug!("Buffer released: {:?}", buffer.id());
            for surface in state.layer_surfaces.values_mut() {
                if let Some(pending) = surface.take_pending_buffer() {
                    if pending.buffer().id() == buffer.id() {
                        pending.set_released(true);
                    }
                }
            }
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        surface: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                debug!(
                    "Layer surface configure event - id: {:?}, size: {}x{}",
                    surface.id(),
                    width,
                    height
                );

                if let Some(layer_surface) = state
                    .layer_surfaces
                    .values_mut()
                    .find(|s| s.layer().id() == surface.id())
                {
                    layer_surface.handle_configure(serial, qh);
                } else {
                    debug!("No matching layer surface found for id: {:?}", surface.id());
                }
            }
            zwlr_layer_surface_v1::Event::Closed => {
                debug!("Layer surface closed: {:?}", surface.id());
                state
                    .layer_surfaces
                    .retain(|_, s| s.layer().id() != surface.id());
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for WaylandState {
    fn event(
        state: &mut Self,
        callback: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { callback_data: _ } = event {
            debug!("Frame callback completed: {:?}", callback.id());
            for surface in state.layer_surfaces.values_mut() {
                surface.handle_frame(callback, qh);
            }
        }
    }
}

impl Dispatch<wp_viewport::WpViewport, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wp_viewport::WpViewport,
        _: wp_viewport::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_viewporter::WpViewporter, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wp_viewporter::WpViewporter,
        _: wp_viewporter::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
