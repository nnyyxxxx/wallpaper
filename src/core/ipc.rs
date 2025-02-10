use crate::WallpaperResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

const SOCKET_PATH: &str = "/tmp/wallpaper.sock";

#[derive(Serialize, Deserialize)]
pub enum IpcMessage {
    SetWallpaper {
        image: PathBuf,
        monitor: Option<String>,
        scaling: String,
    },
    StopDaemon,
}

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub async fn new() -> WallpaperResult<Self> {
        match UnixListener::bind(SOCKET_PATH) {
            Ok(listener) => Ok(Self { listener }),
            Err(_) => {
                if let Ok(mut stream) = UnixStream::connect(SOCKET_PATH).await {
                    let msg = IpcMessage::StopDaemon;
                    let data = bincode::serialize(&msg)?;
                    stream.write_all(&data).await?;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let _ = tokio::fs::remove_file(SOCKET_PATH).await;
                let listener = UnixListener::bind(SOCKET_PATH)?;
                Ok(Self { listener })
            }
        }
    }

    pub async fn accept(&self) -> WallpaperResult<(UnixStream, IpcMessage)> {
        let (mut stream, _) = self.listener.accept().await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        let msg = bincode::deserialize(&buf)?;
        Ok((stream, msg))
    }
}

pub struct IpcClient;

impl IpcClient {
    pub async fn send_message(msg: &IpcMessage) -> WallpaperResult<()> {
        let mut stream = UnixStream::connect(SOCKET_PATH).await?;
        let data = bincode::serialize(msg)?;
        stream.write_all(&data).await?;
        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(SOCKET_PATH);
    }
}
