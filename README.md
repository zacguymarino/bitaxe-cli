# bitaxe-cli  
A small Rust-based command line application for **reading live Bitaxe miner status** and **remotely restarting the miner**, without exposing configuration details or write-dangerous controls.

This tool intentionally **does not modify frequency, voltage, or fan settings** to prevent unintended remote changes. It is built for safe monitoring + emergency reboot capability.

---

## Features

| Command | Purpose |
|--------|---------|
| `status` | Pretty prints important miner statistics |
| `dashboard` | Dumps JSON from dashboard statistics API |
| `restart` | Sends restart command to Bitaxe |

Reads key telemetry such as:
- Hashrate  
- Core / VR temperatures  
- Power usage  
- Voltage info  
- WiFi status  
- Uptime  

---
