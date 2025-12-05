use std::env;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json;
use anyhow::{Result, bail};

/// Config structure (matches config.toml)
#[derive(Debug, Deserialize)]
struct AppConfig {
    host: Option<String>,
}

/// Simple CLI for Bitaxe AxeOS API (read-only + restart)
#[derive(Parser, Debug)]
#[command(
    name = "bitaxe-cli",
    version,
    about = "CLI to monitor and (optionally) restart a Bitaxe miner"
)]
struct Cli {
    /// Override Bitaxe host (ex: http://192.168.1.123)
    /// Priority: CLI flag > BITAXE_URL env var > config file
    #[arg(long)]
    host: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show system info (hashrate, temps, power, wifi, etc.)
    Status,

    /// Restart the miner
    Restart,
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
        Commands::Restart => restart_miner(&client, &host)?,
    }

    Ok(())
}

/// Try to load ~/.config/bitaxe-cli/config.toml if it exists
fn load_config() -> Result<AppConfig> {
    let mut builder = config::Config::builder();

    if let Some(path) = config_path() {
        if path.exists() {
            builder = builder.add_source(config::File::from(path));
        }
    }

    // If there are no sources, this still builds an empty config,
    // and deserialization into AppConfig (all fields Option) is fine.
    let cfg = builder.build().ok();
    if let Some(cfg) = cfg {
        let app_cfg: AppConfig = cfg.try_deserialize()?;
        Ok(app_cfg)
    } else {
        Ok(AppConfig { host: None })
    }
}

/// Build the config file path: ~/.config/bitaxe-cli/config.toml
fn config_path() -> Option<PathBuf> {
    // Cross-platform home dir (HOME on Linux/Mac, USERPROFILE on Windows)
    let home = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"))?;
    let path = PathBuf::from(home).join(".config").join("bitaxe-cli").join("config.toml");
    Some(path)
}

/// Decide which host to use: CLI > BITAXE_URL env > config file
fn resolve_host(cli: &Cli, cfg: &AppConfig) -> Result<String> {
    if let Some(h) = &cli.host {
        return Ok(h.to_string());
    }

    if let Ok(h) = env::var("BITAXE_URL") {
        if !h.is_empty() {
            return Ok(h);
        }
    }

    if let Some(h) = &cfg.host {
        return Ok(h.to_string());
    }

    bail!("No host configured. Use --host, set BITAXE_URL, or create ~/.config/bitaxe-cli/config.toml");
}

fn get_number(root: &serde_json::Value, key: &str) -> Option<f64> {
    root.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_i64().map(|i| i as f64))
            .or_else(|| v.as_u64().map(|u| u as f64))
    })
}

fn get_str<'a>(root: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    root.get(key).and_then(|v| v.as_str())
}

fn get_any_as_string(root: &serde_json::Value, key: &str) -> Option<String> {
    let v = root.get(key)?;
    if let Some(s) = v.as_str() {
        Some(s.to_string())
    } else if let Some(n) = v.as_f64() {
        Some(format!("{}", n))
    } else if let Some(i) = v.as_i64() {
        Some(format!("{}", i))
    } else if let Some(u) = v.as_u64() {
        Some(format!("{}", u))
    } else {
        None
    }
}

fn show_status(client: &Client, host: &str) -> Result<()> {
    let url = format!("{host}/api/system/info");
    let resp = client.get(&url).send()?;
    if !resp.status().is_success() {
        bail!("Request failed with status {}", resp.status());
    }

    let info: serde_json::Value = resp.json()?;

    println!("=== Bitaxe System Info ===");

    // Hostname
    if let Some(hostname) = get_str(&info, "hostname") {
        println!("Hostname        : {hostname}");
    }

    // Hashing
    if let Some(hash) = get_number(&info, "hashRate") {
        println!("Hashrate        : {:.2} GH/s", hash);
    }
    if let Some(best) = get_any_as_string(&info, "bestDiff") {
    println!("Best Diff       : {best}");
    }
    if let Some(best_session) = get_any_as_string(&info, "bestSessionDiff") {
        println!("Best Session    : {best_session}");
    }
    if let Some(accepted) = get_number(&info, "sharesAccepted") {
        println!("Shares Accepted : {:.0}", accepted);
    }
    if let Some(rejected) = get_number(&info, "sharesRejected") {
        println!("Shares Rejected : {:.0}", rejected);
    }

    // Temps
    if let Some(temp) = get_number(&info, "temp") {
        println!("Core Temp       : {:.1} °C", temp);
    }
    if let Some(vr) = get_number(&info, "vrTemp") {
        println!("VR Temp         : {:.1} °C", vr);
    }

    // Power
    if let Some(power) = get_number(&info, "power") {
        println!("Power           : {:.2} W", power);
    }

    if let Some(v_raw) = get_number(&info, "voltage") {
        let v = v_raw / 1000.0;
        println!("PSU Voltage     : {:.2} V", v);
    }

    // Frequency + voltage
    if let Some(freq) = get_number(&info, "frequency") {
        println!("Frequency       : {:.0} MHz", freq);
    }
    if let Some(cv) = get_number(&info, "coreVoltage") {
        println!("Core V (set)    : {:.0} mV", cv);
    }
    if let Some(cva) = get_number(&info, "coreVoltageActual") {
        println!("Core V (actual) : {:.0} mV", cva);
    }

    // Network
    if let Some(rssi) = get_number(&info, "wifiRSSI") {
        println!("WiFi RSSI       : {:.0} dBm", rssi);
    }
    if let Some(status) = get_str(&info, "wifiStatus") {
        println!("WiFi Status     : {status}");
    }

    Ok(())
}

fn restart_miner(client: &Client, host: &str) -> Result<()> {
    let url = format!("{host}/api/system/restart");
    let resp = client.post(&url).send()?;
    if !resp.status().is_success() {
        bail!("Restart failed with status {}", resp.status());
    }
    println!("Restart command sent successfully.");
    Ok(())
}
