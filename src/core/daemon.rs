use crate::{
    core::ipc::{IpcMessage, IpcServer},
    App, WallpaperResult,
};
use parking_lot::Mutex;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct Daemon {
    running: Arc<AtomicBool>,
    server: IpcServer,
    app: Arc<Mutex<App>>,
}

impl Daemon {
    pub async fn new() -> WallpaperResult<Self> {
        let app = App::new()?;

        Ok(Self {
            running: Arc::new(AtomicBool::new(true)),
            server: IpcServer::new().await?,
            app: Arc::new(Mutex::new(app)),
        })
    }

    pub async fn run(&self) -> WallpaperResult<()> {
        while self.running.load(Ordering::Relaxed) {
            let (_, msg) = self.server.accept().await?;
            let mut app = self.app.lock();

            match msg {
                IpcMessage::SetWallpaper { image, monitor: _ } => {
                    app.set_wallpaper_and_exit(image.to_str().unwrap())?;
                }
                IpcMessage::StopDaemon => {
                    self.running.store(false, Ordering::Relaxed);
                }
            }
        }
        Ok(())
    }
}
