/// Monitor enumeration for launcher GUI.
///
/// Provides platform-specific monitor discovery so the video panel can offer
/// a dropdown instead of free-text input. Returns an empty list on failure,
/// which causes the UI to fall back to the existing text field.

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub position: (i32, i32),
    pub size: (u32, u32),
}

impl MonitorInfo {
    pub fn display_label(&self) -> String {
        format!(
            "{} ({}x{} at {},{})",
            self.name, self.size.0, self.size.1, self.position.0, self.position.1
        )
    }

    pub fn to_config_string(&self) -> String {
        self.name.clone()
    }
}

/// Enumerate connected monitors. Returns an empty Vec on unsupported platforms
/// or when enumeration fails.
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    platform::enumerate_monitors_impl()
}

#[cfg(any(target_os = "linux", test))]
fn parse_xrandr_geometry_token(geom: &str) -> Option<((u32, u32), (i32, i32))> {
    let x_pos = geom.find('x')?;
    let width = geom[..x_pos].parse::<u32>().ok()?;
    let rest = &geom[x_pos + 1..];

    let first_sign = rest.find(['+', '-'])?;
    let height = rest[..first_sign].parse::<u32>().ok()?;
    let offsets = &rest[first_sign..];

    let second_sign = offsets[1..].find(['+', '-'])? + 1;
    let x = offsets[..second_sign].parse::<i32>().ok()?;
    let y = offsets[second_sign..].parse::<i32>().ok()?;

    Some(((width, height), (x, y)))
}

#[cfg(target_os = "macos")]
mod platform {
    use super::MonitorInfo;
    use core_graphics::display::{CGDisplay, CGRect};

    pub fn enumerate_monitors_impl() -> Vec<MonitorInfo> {
        let Ok(display_ids) = CGDisplay::active_displays() else {
            return Vec::new();
        };

        display_ids
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                let display = CGDisplay::new(id);
                let bounds: CGRect = display.bounds();
                let name = if display.is_main() {
                    "Main Display".to_string()
                } else {
                    format!("Display {}", i + 1)
                };
                MonitorInfo {
                    name,
                    position: (bounds.origin.x as i32, bounds.origin.y as i32),
                    size: (bounds.size.width as u32, bounds.size.height as u32),
                }
            })
            .collect()
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::MonitorInfo;
    use std::mem;
    use windows::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        DISPLAY_DEVICE_ACTIVE, DISPLAY_DEVICEW, EnumDisplayDevicesW, EnumDisplayMonitors,
        GetMonitorInfoW, HDC, MONITORINFOEXW,
    };

    unsafe extern "system" fn monitor_enum_proc(
        hmonitor: windows::Win32::Graphics::Gdi::HMONITOR,
        _hdc: HDC,
        _lprect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let monitors = &mut *(lparam.0 as *mut Vec<MonitorInfo>);

        let mut info: MONITORINFOEXW = mem::zeroed();
        info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(hmonitor, &mut info as *mut MONITORINFOEXW as *mut _).as_bool() {
            let rc = info.monitorInfo.rcMonitor;
            let device_name = utf16z_to_string(&info.szDevice);

            // Try to get a friendly name via EnumDisplayDevices
            let friendly = {
                let mut dd: DISPLAY_DEVICEW = mem::zeroed();
                dd.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if EnumDisplayDevicesW(windows::core::PCWSTR(info.szDevice.as_ptr()), 0, &mut dd, 0)
                    .as_bool()
                    && (dd.StateFlags & DISPLAY_DEVICE_ACTIVE.0) != 0
                {
                    let s = utf16z_to_string(&dd.DeviceString);
                    if s.is_empty() { device_name.clone() } else { s }
                } else {
                    device_name
                }
            };

            monitors.push(MonitorInfo {
                name: friendly,
                position: (rc.left, rc.top),
                size: ((rc.right - rc.left) as u32, (rc.bottom - rc.top) as u32),
            });
        }

        TRUE
    }

    fn utf16z_to_string(raw: &[u16]) -> String {
        let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
        String::from_utf16_lossy(&raw[..end])
    }

    pub fn enumerate_monitors_impl() -> Vec<MonitorInfo> {
        let mut monitors: Vec<MonitorInfo> = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(
                HDC::default(),
                None,
                Some(monitor_enum_proc),
                LPARAM(&mut monitors as *mut Vec<MonitorInfo> as isize),
            );
        }
        monitors
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::MonitorInfo;
    use std::process::Command;

    pub fn enumerate_monitors_impl() -> Vec<MonitorInfo> {
        let output = match Command::new("xrandr").arg("--query").output() {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => return Vec::new(),
        };

        let mut monitors = Vec::new();
        for line in output.lines() {
            // Lines like: "HDMI-1 connected primary 1920x1080+0+0 ..."
            //             "DP-1 connected 2560x1440+1920+0 ..."
            if !line.contains(" connected") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            let name = parts.first().unwrap_or(&"Unknown").to_string();

            // Find the geometry token: WxH+X+Y
            let geom = parts.iter().find(|p| {
                p.contains('x')
                    && p.contains('+')
                    && p.chars().next().is_some_and(|c| c.is_ascii_digit())
            });

            if let Some(geom) = geom {
                if let Some(info) = parse_geometry(&name, geom) {
                    monitors.push(info);
                }
            }
        }
        monitors
    }

    fn parse_geometry(name: &str, geom: &str) -> Option<MonitorInfo> {
        // Format: WxH+X+Y (xrandr may also use negative offsets)
        let ((w, h), (x, y)) = parse_xrandr_geometry_token(geom)?;
        Some(MonitorInfo {
            name: name.to_string(),
            position: (x, y),
            size: (w, h),
        })
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod platform {
    use super::MonitorInfo;

    pub fn enumerate_monitors_impl() -> Vec<MonitorInfo> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_label_format() {
        let info = MonitorInfo {
            name: "Main Display".to_string(),
            position: (0, 0),
            size: (1920, 1080),
        };
        assert_eq!(info.display_label(), "Main Display (1920x1080 at 0,0)");
    }

    #[test]
    fn display_label_with_offset() {
        let info = MonitorInfo {
            name: "DP-1".to_string(),
            position: (1920, 0),
            size: (2560, 1440),
        };
        assert_eq!(info.display_label(), "DP-1 (2560x1440 at 1920,0)");
    }

    #[test]
    fn to_config_string_returns_name() {
        let info = MonitorInfo {
            name: "HDMI-1".to_string(),
            position: (0, 0),
            size: (3840, 2160),
        };
        assert_eq!(info.to_config_string(), "HDMI-1");
    }

    #[test]
    fn enumerate_monitors_does_not_panic() {
        // Platform-dependent: just ensure no panic.
        let monitors = enumerate_monitors();
        // On CI with no display, this may be empty — that's fine.
        for m in &monitors {
            // Ensure display_label doesn't panic either
            let _ = m.display_label();
            let _ = m.to_config_string();
        }
    }

    #[test]
    fn parse_xrandr_geometry_positive_offsets() {
        assert_eq!(
            parse_xrandr_geometry_token("1920x1080+0+0"),
            Some(((1920, 1080), (0, 0)))
        );
    }

    #[test]
    fn parse_xrandr_geometry_negative_x_offset() {
        assert_eq!(
            parse_xrandr_geometry_token("1920x1080-1920+0"),
            Some(((1920, 1080), (-1920, 0)))
        );
    }
}
