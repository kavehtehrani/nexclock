/// System statistics data for display.
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cpu_temp: Option<f64>,
    pub memory_used_mb: Option<u64>,
    pub memory_total_mb: Option<u64>,
    pub uptime: Option<String>,
}

/// Reads system stats from /proc and /sys. Returns None fields on non-Linux
/// or if the files are unavailable.
pub fn read_system_stats() -> SystemStats {
    SystemStats {
        cpu_temp: read_cpu_temp(),
        memory_used_mb: read_memory().map(|(used, _)| used),
        memory_total_mb: read_memory().map(|(_, total)| total),
        uptime: read_uptime(),
    }
}

/// Reads CPU temperature, trying multiple sysfs sources.
fn read_cpu_temp() -> Option<f64> {
    // Try thermal_zone entries first (common on Raspberry Pi and laptops)
    if let Some(temp) = read_thermal_zone() {
        return Some(temp);
    }

    // Try hwmon entries (common on desktops)
    read_hwmon_temp()
}

/// Reads from /sys/class/thermal/thermal_zone*/temp
fn read_thermal_zone() -> Option<f64> {
    let thermal_dir = std::fs::read_dir("/sys/class/thermal/").ok()?;
    for entry in thermal_dir.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("thermal_zone")
            && let Ok(content) = std::fs::read_to_string(entry.path().join("temp"))
            && let Ok(millidegrees) = content.trim().parse::<f64>()
        {
            return Some(millidegrees / 1000.0);
        }
    }
    None
}

/// Reads from /sys/class/hwmon/hwmon*/temp*_input
fn read_hwmon_temp() -> Option<f64> {
    let hwmon_dir = std::fs::read_dir("/sys/class/hwmon/").ok()?;
    for entry in hwmon_dir.flatten() {
        let dir = entry.path();
        let files = std::fs::read_dir(&dir).ok()?;
        for file in files.flatten() {
            let fname = file.file_name();
            let fname_str = fname.to_string_lossy();
            if fname_str.starts_with("temp")
                && fname_str.ends_with("_input")
                && let Ok(content) = std::fs::read_to_string(file.path())
                && let Ok(millidegrees) = content.trim().parse::<f64>()
            {
                return Some(millidegrees / 1000.0);
            }
        }
    }
    None
}

/// Reads memory info from /proc/meminfo. Returns (used_mb, total_mb).
fn read_memory() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb = None;
    let mut available_kb = None;

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("MemTotal:") {
            total_kb = parse_meminfo_value(val);
        } else if let Some(val) = line.strip_prefix("MemAvailable:") {
            available_kb = parse_meminfo_value(val);
        }
        if total_kb.is_some() && available_kb.is_some() {
            break;
        }
    }

    let total = total_kb?;
    let available = available_kb?;
    let used = total.saturating_sub(available);
    Some((used / 1024, total / 1024))
}

/// Parses a /proc/meminfo value line like "  16384 kB" into kB.
fn parse_meminfo_value(val: &str) -> Option<u64> {
    val.split_whitespace().next()?.parse().ok()
}

/// Reads system uptime from /proc/uptime and formats it as "Xd Xh Xm".
fn read_uptime() -> Option<String> {
    let content = std::fs::read_to_string("/proc/uptime").ok()?;
    let secs: f64 = content.split_whitespace().next()?.parse().ok()?;
    let total_secs = secs as u64;

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;

    if days > 0 {
        Some(format!("{days}d {hours}h {minutes}m"))
    } else if hours > 0 {
        Some(format!("{hours}h {minutes}m"))
    } else {
        Some(format!("{minutes}m"))
    }
}
