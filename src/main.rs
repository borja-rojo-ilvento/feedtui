mod app;
mod config;
mod event;
mod feeds;
mod ui;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "feedtui")]
#[command(about = "A configurable terminal dashboard for stocks, news, sports, and social feeds")]
struct Args {
    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load config
    let config_path = args.config.unwrap_or_else(|| {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("feedtui")
            .join("config.toml")
    });

    let config = config::Config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("Warning: Could not load config from {:?}: {}", config_path, e);
        eprintln!("Using default configuration...");
        config::Config::default()
    });

    // Run the app
    let mut app = app::App::new(config);
    app.run().await
}
