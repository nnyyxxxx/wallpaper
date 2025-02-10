use wayland_client::protocol::wl_output::WlOutput;

#[derive(Debug, Clone)]
pub struct Monitor {
    pub output: WlOutput,
    pub name: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
    pub scale: f64,
}

impl Monitor {
    pub fn new(output: WlOutput, name: u32) -> Self {
        Self {
            output,
            name,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            refresh: 0,
            scale: 1.0,
        }
    }
}

pub struct MonitorBuilder {
    name: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    refresh: i32,
    output: WlOutput,
}

impl MonitorBuilder {
    pub fn new(name: u32, output: WlOutput) -> Self {
        Self {
            name,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            refresh: 0,
            output,
        }
    }

    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn refresh(mut self, refresh: i32) -> Self {
        self.refresh = refresh;
        self
    }

    pub fn build(self) -> Monitor {
        Monitor {
            output: self.output,
            name: self.name,
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            refresh: self.refresh,
            scale: 1.0,
        }
    }
}
impl Monitor {
    pub fn physical_size(&self) -> (u32, u32) {
        (
            (self.width as f64 * self.scale) as u32,
            (self.height as f64 * self.scale) as u32,
        )
    }
}
