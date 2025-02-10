use crate::{
    core::buffer::Buffer,
    display::monitor::Monitor,
    utils::{error::WallpaperResult, wayland::WaylandState},
};
use log::debug;
use wayland_client::{
    protocol::{wl_callback, wl_compositor, wl_surface},
    Connection, Proxy, QueueHandle,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

#[derive(Clone)]
pub struct LayerSurface {
    surface: wl_surface::WlSurface,
    layer: ZwlrLayerSurfaceV1,
    viewport: Option<wp_viewport::WpViewport>,
    configured: bool,
    pending_buffer: Option<Buffer>,
    frame_callback: Option<wl_callback::WlCallback>,
    frame_done: bool,
}

impl LayerSurface {
    pub fn new(
        _: &Connection,
        qh: &QueueHandle<WaylandState>,
        monitor: &Monitor,
        layer_shell: &ZwlrLayerShellV1,
        compositor: &wl_compositor::WlCompositor,
        viewporter: Option<&wp_viewporter::WpViewporter>,
    ) -> WallpaperResult<Self> {
        let surface = compositor.create_surface(qh, ());
        debug!("Created wayland surface: {:?}", surface.id());

        let viewport = viewporter.map(|v| v.get_viewport(&surface, qh, ()));

        let layer = layer_shell.get_layer_surface(
            &surface,
            Some(&monitor.output),
            zwlr_layer_shell_v1::Layer::Background,
            "wallpaper".to_string(),
            qh,
            (),
        );

        debug!("Setting layer surface properties");
        layer.set_size(0, 0);
        layer.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
        let region = compositor.create_region(qh, ());
        surface.set_input_region(Some(&region));
        surface.set_buffer_scale(1);

        let frame_callback = Some(surface.frame(qh, ()));

        surface.commit();

        Ok(Self {
            surface,
            layer,
            viewport,
            configured: false,
            pending_buffer: None,
            frame_callback,
            frame_done: true,
        })
    }

    pub fn attach_buffer(&mut self, buffer: &Buffer, _qh: &QueueHandle<WaylandState>) {
        self.pending_buffer = Some(buffer.clone());
        self.surface.attach(Some(&buffer.buffer()), 0, 0);

        if let Some(viewport) = &self.viewport {
            let (width, height) = buffer.size();
            viewport.set_destination(width as i32, height as i32);
        }

        self.surface.commit();
    }

    pub fn surface(&self) -> &wl_surface::WlSurface {
        &self.surface
    }

    pub fn layer(&self) -> &ZwlrLayerSurfaceV1 {
        &self.layer
    }

    pub fn handle_configure(&mut self, serial: u32, qh: &QueueHandle<WaylandState>) {
        debug!("Handling configure for surface {:?}", self.surface.id());
        self.layer.ack_configure(serial);

        if !self.configured {
            self.configured = true;
            if let Some(buffer) = self.pending_buffer.take() {
                debug!("Applying pending buffer after configure");
                self.attach_buffer(&buffer, qh);
            }
        }
    }

    pub fn handle_frame(
        &mut self,
        _callback: &wl_callback::WlCallback,
        qh: &QueueHandle<WaylandState>,
    ) {
        self.frame_done = true;
        if self.pending_buffer.is_some() {
            self.frame_callback = Some(self.surface.frame(qh, ()));
            self.frame_done = false;
            self.surface.commit();
        }
    }

    pub fn is_draw_ready(&self) -> bool {
        self.frame_done && self.configured
    }

    pub fn is_configured(&self) -> bool {
        self.configured
    }

    pub fn set_configured(&mut self, configured: bool) {
        self.configured = configured;
    }

    pub fn take_pending_buffer(&mut self) -> Option<Buffer> {
        self.pending_buffer.take()
    }

    pub fn needs_redraw(&self) -> bool {
        self.pending_buffer.is_some() && self.frame_done
    }
}
