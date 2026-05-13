use std::process::Command;

use serde_json::Value;
use sysinfo::System;

use crate::runtime::{ComputeBackend, DeviceProfile, DeviceTier, GraphicsDevice};

const GB: u64 = 1024 * 1024 * 1024;

/// Get system memory information in bytes.
pub fn get_memory_info() -> (u64, u64) {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total = sys.total_memory();
    let available = sys.available_memory();

    (total, available)
}

/// Check if system has enough available memory for a model requirement.
pub fn check_memory_requirements(required_mb: u64) -> bool {
    let (_, available) = get_memory_info();
    let available_mb = available / 1024 / 1024;
    available_mb >= required_mb
}

pub fn detect_device_profile() -> DeviceProfile {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_memory_bytes = sys.total_memory();
    let available_memory_bytes = sys.available_memory();
    let graphics = detect_graphics_devices();
    let tier = classify_device_tier(total_memory_bytes, &graphics);

    DeviceProfile {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        cpu_model: sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string()),
        physical_cores: sys
            .physical_core_count()
            .unwrap_or_else(|| sys.cpus().len().max(1)),
        logical_cores: sys.cpus().len().max(1),
        total_memory_bytes,
        available_memory_bytes,
        battery_powered: None,
        thermal_class: None,
        graphics,
        tier,
    }
}

pub fn classify_device_tier(total_memory_bytes: u64, graphics: &[GraphicsDevice]) -> DeviceTier {
    let has_strong_dgpu = graphics
        .iter()
        .any(|device| !device.integrated && device.vram_bytes.unwrap_or_default() >= 6 * GB);

    if total_memory_bytes < 8 * GB {
        DeviceTier::Low
    } else if total_memory_bytes < 16 * GB {
        DeviceTier::Balanced
    } else if total_memory_bytes < 32 * GB {
        if has_strong_dgpu {
            DeviceTier::Workstation
        } else {
            DeviceTier::Performance
        }
    } else {
        DeviceTier::Workstation
    }
}

fn detect_graphics_devices() -> Vec<GraphicsDevice> {
    let mut devices = Vec::new();

    if let Some(nvidia_devices) = detect_nvidia_devices() {
        devices.extend(nvidia_devices);
    }

    #[cfg(target_os = "windows")]
    {
        if devices.is_empty() {
            devices.extend(detect_windows_video_devices());
        }
    }

    #[cfg(target_os = "linux")]
    {
        if devices.is_empty() {
            devices.extend(detect_linux_video_devices());
        }
    }

    #[cfg(target_os = "macos")]
    {
        if devices.is_empty() {
            devices.extend(detect_macos_video_devices());
        }
    }

    dedupe_graphics_devices(devices)
}

fn dedupe_graphics_devices(devices: Vec<GraphicsDevice>) -> Vec<GraphicsDevice> {
    let mut unique = Vec::new();

    for device in devices {
        let already_present = unique.iter().any(|existing: &GraphicsDevice| {
            existing.name == device.name && existing.vendor == device.vendor
        });
        if !already_present {
            unique.push(device);
        }
    }

    unique
}

fn detect_nvidia_devices() -> Option<Vec<GraphicsDevice>> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let devices = stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split(',').map(|part| part.trim());
            let name = parts.next()?.to_string();
            let memory_mb = parts.next()?.parse::<u64>().ok();

            Some(GraphicsDevice {
                name,
                vendor: Some("nvidia".to_string()),
                vram_bytes: memory_mb.map(|value| value * 1024 * 1024),
                integrated: false,
                backends: vec![ComputeBackend::Cuda, ComputeBackend::Vulkan],
            })
        })
        .collect::<Vec<_>>();

    Some(devices)
}

#[cfg(target_os = "windows")]
fn detect_windows_video_devices() -> Vec<GraphicsDevice> {
    let script = "Get-CimInstance Win32_VideoController | Select-Object Name, AdapterRAM | ConvertTo-Json -Compress";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    parse_windows_gpu_json(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "windows")]
fn parse_windows_gpu_json(json: &str) -> Vec<GraphicsDevice> {
    let Ok(value) = serde_json::from_str::<Value>(json.trim()) else {
        return Vec::new();
    };

    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(graphics_device_from_windows_value)
            .collect(),
        Value::Object(_) => graphics_device_from_windows_value(&value)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
fn graphics_device_from_windows_value(value: &Value) -> Option<GraphicsDevice> {
    let name = value.get("Name")?.as_str()?.to_string();
    let adapter_ram = value.get("AdapterRAM").and_then(Value::as_u64);
    Some(build_graphics_device(name, adapter_ram))
}

#[cfg(target_os = "linux")]
fn detect_linux_video_devices() -> Vec<GraphicsDevice> {
    let output = Command::new("lspci").output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains("VGA") || line.contains("3D controller"))
        .map(|line| build_graphics_device(line.to_string(), None))
        .collect()
}

#[cfg(target_os = "macos")]
fn detect_macos_video_devices() -> Vec<GraphicsDevice> {
    let output = Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        return Vec::new();
    };

    value
        .get("SPDisplaysDataType")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let name = item
                .get("sppci_model")
                .and_then(Value::as_str)
                .or_else(|| item.get("_name").and_then(Value::as_str))?;
            Some(build_graphics_device(name.to_string(), None))
        })
        .collect()
}

fn build_graphics_device(name: String, vram_bytes: Option<u64>) -> GraphicsDevice {
    let lower = name.to_lowercase();
    let vendor = if lower.contains("nvidia") {
        Some("nvidia".to_string())
    } else if lower.contains("amd") || lower.contains("radeon") {
        Some("amd".to_string())
    } else if lower.contains("intel") {
        Some("intel".to_string())
    } else if lower.contains("apple") {
        Some("apple".to_string())
    } else {
        None
    };

    let integrated = lower.contains("intel") || lower.contains("iris") || lower.contains("apple");
    let backends = match vendor.as_deref() {
        Some("nvidia") => vec![ComputeBackend::Cuda, ComputeBackend::Vulkan],
        Some("amd") => vec![ComputeBackend::Rocm, ComputeBackend::Vulkan],
        Some("intel") => vec![ComputeBackend::Vulkan, ComputeBackend::DirectMl],
        Some("apple") => vec![ComputeBackend::Metal, ComputeBackend::CoreMl],
        _ => vec![ComputeBackend::Cpu],
    };

    GraphicsDevice {
        name,
        vendor,
        vram_bytes,
        integrated,
        backends,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_low_tier_machine() {
        let tier = classify_device_tier(6 * GB, &[]);
        assert_eq!(tier, DeviceTier::Low);
    }

    #[test]
    fn classifies_workstation_when_memory_is_high() {
        let tier = classify_device_tier(64 * GB, &[]);
        assert_eq!(tier, DeviceTier::Workstation);
    }
}
