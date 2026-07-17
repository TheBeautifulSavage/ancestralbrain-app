/// Cross-platform hardware detection — RAM, CPU, recommended Ollama model.
use serde::Serialize;
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct HardwareInfo {
    pub arch: String,       // "arm64" or "x86_64"
    pub cpu_brand: String,  // e.g. "Apple M3 Pro"
    pub ram_gb: u64,
    pub chat_model: String,  // recommended Ollama chat model
    pub embed_model: String, // always "nomic-embed-text"
    pub reason: String,      // human-readable explanation shown in UI
}

pub fn detect() -> HardwareInfo {
    let arch = std::env::consts::ARCH.to_string(); // "aarch64" or "x86_64"
    let cpu_brand = detect_cpu_brand();
    let ram_gb = detect_ram_gb();

    // Apple Silicon = macOS + aarch64
    let is_apple_silicon = cfg!(target_os = "macos") && arch == "aarch64";

    let (chat_model, reason) = select_model(is_apple_silicon, ram_gb);

    HardwareInfo {
        arch: if arch == "aarch64" { "arm64".to_string() } else { arch },
        cpu_brand,
        ram_gb,
        chat_model,
        embed_model: "nomic-embed-text".to_string(),
        reason,
    }
}

fn select_model(is_apple_silicon: bool, ram_gb: u64) -> (String, String) {
    if !is_apple_silicon {
        // Intel Mac / Windows / Linux — conservative choice
        if ram_gb >= 16 {
            (
                "llama3.2:3b".to_string(),
                format!("Detected {} GB RAM — using 3B model for good performance", ram_gb),
            )
        } else {
            (
                "llama3.2:1b".to_string(),
                format!("Detected {} GB RAM — using smallest model for best compatibility", ram_gb),
            )
        }
    } else if ram_gb >= 32 {
        (
            "llama3.1:8b".to_string(),
            format!("Apple Silicon + {} GB RAM — using full 8B model for best results", ram_gb),
        )
    } else if ram_gb >= 16 {
        (
            "llama3.2:3b".to_string(),
            format!(
                "Apple Silicon + {} GB RAM — using 3B model, great balance of speed and quality",
                ram_gb
            ),
        )
    } else {
        (
            "llama3.2:1b".to_string(),
            format!(
                "Apple Silicon + {} GB RAM — using 1B model optimized for your machine",
                ram_gb
            ),
        )
    }
}

// ── Platform-specific RAM detection ──────────────────────────────────────────

fn detect_ram_gb() -> u64 {
    #[cfg(target_os = "macos")]
    {
        detect_ram_macos().unwrap_or(8)
    }
    #[cfg(target_os = "windows")]
    {
        detect_ram_windows().unwrap_or(8)
    }
    #[cfg(target_os = "linux")]
    {
        detect_ram_linux().unwrap_or(8)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        8
    }
}

#[cfg(target_os = "macos")]
fn detect_ram_macos() -> Option<u64> {
    let out = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()?;
    let s = String::from_utf8(out.stdout).ok()?;
    let bytes: u64 = s.trim().parse().ok()?;
    Some(bytes / (1024 * 1024 * 1024))
}

#[cfg(target_os = "windows")]
fn detect_ram_windows() -> Option<u64> {
    let out = Command::new("wmic")
        .args(["ComputerSystem", "get", "TotalPhysicalMemory", "/value"])
        .output()
        .ok()?;
    let s = String::from_utf8(out.stdout).ok()?;
    // Output: "TotalPhysicalMemory=17179869184\r\n\r\n"
    for line in s.lines() {
        if let Some(val) = line.strip_prefix("TotalPhysicalMemory=") {
            let bytes: u64 = val.trim().parse().ok()?;
            return Some(bytes / (1024 * 1024 * 1024));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn detect_ram_linux() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    // MemTotal:       16384000 kB
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / (1024 * 1024));
        }
    }
    None
}

// ── CPU brand detection ───────────────────────────────────────────────────────

fn detect_cpu_brand() -> String {
    #[cfg(target_os = "macos")]
    {
        Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Apple Silicon".to_string())
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("wmic")
            .args(["cpu", "get", "Name", "/value"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("Name="))
                    .and_then(|l| l.strip_prefix("Name="))
                    .map(|v| v.trim().to_string())
            })
            .unwrap_or_else(|| "Unknown CPU".to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("model name"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|v| v.trim().to_string())
            })
            .unwrap_or_else(|| "Unknown CPU".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        "Unknown CPU".to_string()
    }
}
