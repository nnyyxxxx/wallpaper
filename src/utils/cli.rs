use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
pub enum Command {
    #[command(name = "set")]
    SetWallpaper {
        /// Path to the wallpaper image
        #[arg(short, long)]
        image: PathBuf,

        /// Monitor to set wallpaper on (default: all)
        #[arg(short, long)]
        monitor: Option<String>,

        /// Scaling mode (fill, fit, stretch)
        #[arg(short, long, default_value = "fill")]
        scaling: String,
    },

    #[command(name = "daemon")]
    Daemon {
        #[arg(short, long)]
        start: bool,
    },
}
