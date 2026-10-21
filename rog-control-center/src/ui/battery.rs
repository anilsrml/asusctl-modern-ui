//! Logic behind the Battery page: sysfs readings and charge-limit presets.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rog_dbus::zbus_platform::PlatformProxy;
use slint::ComponentHandle;

use super::show_toast;
use crate::zbus_proxies::AppState;
use crate::{BatteryPageData, MainWindow};

const POWER_SUPPLY: &str = "/sys/class/power_supply";

/// Charge limits offered as quick presets, in segment order.
pub const PRESETS: [i32; 3] = [
    60, 80, 100,
];

/// Preset segment index for a threshold, `None` when it is not a preset.
pub fn preset_index(threshold: f32) -> Option<usize> {
    PRESETS.iter().position(|p| *p as f32 == threshold)
}

/// Threshold written for a preset segment index.
pub fn preset_threshold(index: usize) -> Option<i32> {
    PRESETS.get(index).copied()
}

/// Charging state as reported by the battery driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeState {
    Charging,
    Discharging,
    NotCharging,
    Full,
}

impl ChargeState {
    /// Code shared with the Slint page (0 means unknown).
    pub fn code(self) -> i32 {
        match self {
            ChargeState::Charging => 1,
            ChargeState::Discharging => 2,
            ChargeState::Full => 3,
            ChargeState::NotCharging => 4,
        }
    }
}

/// Everything the page shows; a field is `None` when sysfs does not provide it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BatteryReading {
    pub percent: Option<i32>,
    pub state: Option<ChargeState>,
    pub power_w: Option<f32>,
    pub health: Option<i32>,
    pub cycles: Option<i32>,
}

/// First `BAT*` supply of type Battery under a power_supply directory.
pub fn find_battery(root: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("BAT"))
        .map(|e| e.path())
        .filter(|p| std::fs::read_to_string(p.join("type")).is_ok_and(|t| t.trim() == "Battery"))
        .collect();
    found.sort();
    found.into_iter().next()
}

fn read_u64(dir: &Path, name: &str) -> Option<u64> {
    std::fs::read_to_string(dir.join(name))
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn parse_state(raw: &str) -> Option<ChargeState> {
    match raw.trim() {
        "Charging" => Some(ChargeState::Charging),
        "Discharging" => Some(ChargeState::Discharging),
        "Not charging" => Some(ChargeState::NotCharging),
        "Full" => Some(ChargeState::Full),
        _ => None,
    }
}

/// Watts from `power_now`, or `current_now` x `voltage_now` (all micro-units).
fn read_power_w(dir: &Path) -> Option<f32> {
    if let Some(uw) = read_u64(dir, "power_now") {
        return Some((uw as f64 / 1e6) as f32);
    }
    let ua = read_u64(dir, "current_now")?;
    let uv = read_u64(dir, "voltage_now")?;
    Some((ua as f64 * uv as f64 / 1e12) as f32)
}

/// Full-capacity ratio in percent; energy and charge units cancel out.
fn read_health(dir: &Path) -> Option<i32> {
    let pair = |full: &str, design: &str| Some((read_u64(dir, full)?, read_u64(dir, design)?));
    let (full, design) = pair("energy_full", "energy_full_design")
        .or_else(|| pair("charge_full", "charge_full_design"))?;
    if design == 0 {
        return None;
    }
    Some((full as f64 / design as f64 * 100.0).round().min(100.0) as i32)
}

/// Read one battery directory without inventing values for missing files.
pub fn read_battery(dir: &Path) -> BatteryReading {
    BatteryReading {
        percent: read_u64(dir, "capacity").map(|v| v.min(100) as i32),
        state: std::fs::read_to_string(dir.join("status"))
            .ok()
            .and_then(|s| parse_state(&s)),
        power_w: read_power_w(dir),
        health: read_health(dir),
        // Many laptops report 0 when the firmware does not track cycles
        cycles: read_u64(dir, "cycle_count")
            .filter(|v| *v > 0)
            .and_then(|v| i32::try_from(v).ok()),
    }
}

/// Wire the Battery page: live sysfs readings and the one-shot full charge.
pub fn setup_battery_page(ui: &MainWindow, app_state: Arc<Mutex<AppState>>) {
    let data = ui.global::<BatteryPageData>();
    data.on_preset_index_for(|threshold| preset_index(threshold).map_or(-1, |i| i as i32));

    let weak = ui.as_weak();
    data.on_one_shot_full_charge(move || {
        let weak = weak.clone();
        tokio::spawn(async move {
            let result = match zbus::Connection::system().await {
                Ok(conn) => match PlatformProxy::new(&conn).await {
                    Ok(platform) => platform.one_shot_full_charge().await,
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            };
            show_toast(
                "Charging to 100% once".into(),
                "One-time full charge failed".into(),
                weak,
                result,
            );
        });
    });

    let weak = ui.as_weak();
    tokio::spawn(async move {
        loop {
            let visible = app_state
                .try_lock()
                .map(|s| *s == AppState::MainWindowOpen)
                .unwrap_or(false);
            if visible {
                let reading = find_battery(Path::new(POWER_SUPPLY))
                    .map(|dir| read_battery(&dir))
                    .unwrap_or_default();
                let shown = weak.upgrade_in_event_loop(move |ui| {
                    let d = ui.global::<BatteryPageData>();
                    d.set_percent(reading.percent.unwrap_or(-1));
                    d.set_state(reading.state.map_or(0, ChargeState::code));
                    d.set_power_w(reading.power_w.unwrap_or(-1.0));
                    d.set_health(reading.health.unwrap_or(-1));
                    d.set_cycles(reading.cycles.unwrap_or(-1));
                });
                if shown.is_err() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn fixture(files: &[(&str, &str)]) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rcc-battery-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        for (name, body) in files {
            fs::write(dir.join(name), body).unwrap();
        }
        dir
    }

    #[test]
    fn presets_map_both_ways() {
        assert_eq!(preset_index(60.0), Some(0));
        assert_eq!(preset_index(80.0), Some(1));
        assert_eq!(preset_index(100.0), Some(2));
        assert_eq!(preset_index(75.0), None);
        assert_eq!(preset_index(-1.0), None);
        assert_eq!(preset_threshold(1), Some(80));
        assert_eq!(preset_threshold(3), None);
    }

    #[test]
    fn full_power_based_battery_reads_every_field() {
        let dir = fixture(&[
            ("capacity", "73\n"),
            ("status", "Discharging\n"),
            ("power_now", "12500000\n"),
            ("energy_full", "45000000\n"),
            ("energy_full_design", "50000000\n"),
            ("cycle_count", "212\n"),
        ]);
        let r = read_battery(&dir);
        assert_eq!(r.percent, Some(73));
        assert_eq!(r.state, Some(ChargeState::Discharging));
        assert_eq!(r.power_w, Some(12.5));
        assert_eq!(r.health, Some(90));
        assert_eq!(r.cycles, Some(212));
    }

    #[test]
    fn charge_based_battery_derives_watts_and_health() {
        let dir = fixture(&[
            ("current_now", "2000000\n"),
            ("voltage_now", "12000000\n"),
            ("charge_full", "4000000\n"),
            ("charge_full_design", "5000000\n"),
        ]);
        let r = read_battery(&dir);
        assert_eq!(r.power_w, Some(24.0));
        assert_eq!(r.health, Some(80));
    }

    #[test]
    fn missing_files_give_none_not_zero() {
        let r = read_battery(&fixture(&[("capacity", "50")]));
        assert_eq!(r.percent, Some(50));
        assert_eq!(r.state, None);
        assert_eq!(r.power_w, None);
        assert_eq!(r.health, None);
        assert_eq!(r.cycles, None);
    }

    #[test]
    fn unparsable_or_degenerate_values_are_none() {
        let r = read_battery(&fixture(&[
            ("status", "Weird"),
            ("power_now", "abc"),
            ("energy_full", "10"),
            ("energy_full_design", "0"),
            ("cycle_count", "-1"),
        ]));
        assert_eq!(r.state, None);
        assert_eq!(r.power_w, None);
        assert_eq!(r.health, None);
        assert_eq!(r.cycles, None);
    }

    #[test]
    fn zero_cycle_count_means_firmware_does_not_report_it() {
        let r = read_battery(&fixture(&[("cycle_count", "0\n")]));
        assert_eq!(r.cycles, None);
    }

    #[test]
    fn health_is_capped_at_100() {
        let r = read_battery(&fixture(&[
            ("energy_full", "52"),
            ("energy_full_design", "50"),
        ]));
        assert_eq!(r.health, Some(100));
    }

    #[test]
    fn find_battery_skips_mains_and_non_bat() {
        let root = fixture(&[]);
        for (name, ty) in [
            ("AC0", "Mains"),
            ("BAT0", "Battery"),
            ("hidpp", "Battery"),
        ] {
            fs::create_dir_all(root.join(name)).unwrap();
            fs::write(root.join(name).join("type"), ty).unwrap();
        }
        assert_eq!(find_battery(&root), Some(root.join("BAT0")));
        assert_eq!(find_battery(&fixture(&[])), None);
    }
}
