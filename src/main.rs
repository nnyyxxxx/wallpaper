use clap::Parser;
use wallpaper::{
    core::{
        daemon::Daemon,
        ipc::{IpcClient, IpcMessage},
    },
    utils::cli::{Cli, Command},
    WallpaperResult,
};

#[tokio::main]
async fn main() -> WallpaperResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::SetWallpaper {
            image,
            monitor,
            scaling,
        } => {
            let msg = IpcMessage::SetWallpaper {
                image,
                monitor,
                scaling,
            };
            IpcClient::send_message(&msg).await?;
        }
        Command::Daemon { start } => {
            if start {
                let daemon = Daemon::new().await?;
                daemon.run().await?;
            } else {
                IpcClient::send_message(&IpcMessage::StopDaemon).await?;
            }
        }
    }

    Ok(())
}
