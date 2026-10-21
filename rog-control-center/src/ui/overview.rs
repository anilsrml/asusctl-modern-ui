//! Logic behind the Overview page.

use slint::{ComponentHandle, Model};

use crate::{MainWindow, OverviewData, SystemPageData, types::profile_map};

/// Charge end-threshold written when the limiter is switched on.
pub const CHARGE_LIMIT_ON: i32 = 80;
/// Charge end-threshold written when the limiter is switched off.
pub const CHARGE_LIMIT_OFF: i32 = 100;

/// Threshold to write for the "Limit charge to 80%" switch.
pub fn charge_limit_for(enabled: bool) -> i32 {
    if enabled {
        CHARGE_LIMIT_ON
    } else {
        CHARGE_LIMIT_OFF
    }
}

/// Whether a threshold read from the daemon counts as "limited".
/// Unsupported (-1) is never limited.
pub fn is_charge_limited(threshold: f32) -> bool {
    threshold >= 0.0 && threshold < CHARGE_LIMIT_OFF as f32
}

/// Parse a sysfs `capacity` value (percent) into 0..=100, `None` if unreadable.
pub fn parse_capacity(raw: &str) -> Option<i32> {
    raw.trim().parse::<i32>().ok().map(|v| v.clamp(0, 100))
}

/// Current charge of the first battery under /sys/class/power_supply.
pub fn read_battery_percent() -> Option<i32> {
    std::fs::read_dir("/sys/class/power_supply")
        .ok()?
        .flatten()
        .filter(|e| {
            std::fs::read_to_string(e.path().join("type")).is_ok_and(|t| t.trim() == "Battery")
        })
        .find_map(|e| parse_capacity(&std::fs::read_to_string(e.path().join("capacity")).ok()?))
}

/// Route the Overview charge-limit switch through the System page callback and
/// resolve profile selector positions from the offered profile list.
pub fn setup_overview_page(ui: &MainWindow) {
    let weak = ui.as_weak();
    ui.global::<OverviewData>()
        .on_profile_position(move |value| {
            weak.upgrade().map_or(-1, |ui| {
                let indexes: Vec<i32> = ui
                    .global::<SystemPageData>()
                    .get_platform_profile_indexes()
                    .iter()
                    .collect();
                profile_map::slint_position(&indexes, value)
            })
        });
    let weak = ui.as_weak();
    ui.global::<OverviewData>()
        .on_toggle_charge_limit(move |enabled| {
            let value = charge_limit_for(enabled);
            if let Some(ui) = weak.upgrade() {
                let data = ui.global::<SystemPageData>();
                data.set_charge_control_end_threshold(value as f32);
                data.invoke_cb_charge_control_end_threshold(value);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_parses_and_clamps() {
        assert_eq!(parse_capacity("73\n"), Some(73));
        assert_eq!(parse_capacity("130"), Some(100));
        assert_eq!(parse_capacity("-4"), Some(0));
        assert_eq!(parse_capacity("abc"), None);
    }

    #[test]
    fn toggle_maps_to_80_or_100() {
        assert_eq!(charge_limit_for(true), 80);
        assert_eq!(charge_limit_for(false), 100);
    }

    #[test]
    fn threshold_below_full_is_limited() {
        assert!(is_charge_limited(80.0));
        assert!(is_charge_limited(60.0));
        assert!(!is_charge_limited(100.0));
        assert!(!is_charge_limited(-1.0));
    }
}
