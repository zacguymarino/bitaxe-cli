use clap::{Parser, Subcommand};
use reqwest::blocking::Client;
use serde::Deserialize;
use anyhow::Result;

/// Config structure (matches config.toml)
#[derive(Debug, Deserialize)]
struct AppConfig {
    host: Option<String>,
}

/// Simple CLI for Bitaxe AxeOS API
#[derive(Parser, Debug)]
#[command(name = "bitaxe-cli", version, about = "Bitaxe monitor & restart CLI")]
struct Cli {
    /// Override Bitaxe host (ex: http://192.168.1.123)
    /// Priority: CLI > ENV > config.toml
    #[arg(short, long, env = "BITAXE_URL")]
    host: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Status,
    Dashboard,
    Restart,
}

fn load_config() -> Result<AppConfig> {
    let mut settings = config::Config::builder();

    // Try ~/.config/bitaxe-cli/config.toml
    if let Some(home) = dirs::home_dir() {
        let config_path = home.join(".config/bitaxe-cli/config.toml");
        if config_path.exists() {
            settings = settings.add_source(config::File::from(config_path));
        }
    }

    Ok(settings.build()?.try_deserialize::<AppConfig>()?)
}

fn resolve_host(cli: &Cli, cfg: &AppConfig) -> Result<String> {
    if let Some(h) = &cli.host {
        return Ok(h.to_string());
    }
    if let Some(h) = &cfg.host {
        return Ok(h.to_string());
    }
    anyhow::bail!("No host configured. Use --host or create ~/.config/bitaxe-cli/config.toml");
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = load_config().unwrap_or(AppConfig { host: None });

    let host = resolve_host(&cli, &cfg)?;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    match cli.command {
        Commands::Status => show_status(&client, &host)?,
        Commands::Dashboard => show_dashboard(&client, &host)?,
        Commands::Restart => restart_miner(&client, &host)?,
    }

    Ok(())
}
