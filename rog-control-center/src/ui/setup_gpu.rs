use std::sync::Arc;

use log::{error, info};
use rog_dbus::asus_armoury::AsusArmouryProxy;
use rog_dbus::zbus_platform::PlatformProxy;
use rog_dbus::zbus_xgm_led::XgmLedProxy;
use rog_platform::asus_armoury::FirmwareAttribute;
use slint::{ComponentHandle, SharedString, Weak};

use super::show_toast;
use crate::zbus_proxies::find_iface_async;
use crate::{GPUPageData, MainWindow};

/// A selectable GPU mode, independent of which sysfs attributes back it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GpuMode {
    Integrated, // dGPU powered off
    Hybrid,     // iGPU + dGPU (Optimus)
    Ultimate,   // dGPU drives the displays directly (MUX)
}

impl GpuMode {
    /// Stable id shared with the Slint page (which owns the translated labels).
    fn id(self) -> i32 {
        match self {
            GpuMode::Integrated => 0,
            GpuMode::Hybrid => 1,
            GpuMode::Ultimate => 2,
        }
    }
}

/// Result of a mode change as the UI should show it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ModeView {
    shown: GpuMode,
    reboot_pending: bool,
}

/// Decide what the segmented control shows after a write attempt.
///
/// `boot` is the mode active when the app started (the firmware applies a
/// change only after a reboot). A successful write shows the requested mode and
/// flags a reboot when it differs from `boot`; a failed write falls back to the
/// hardware `readback`, with a reboot flagged only if that differs from `boot`.
fn mode_after_apply(boot: GpuMode, target: GpuMode, write_ok: bool, readback: GpuMode) -> ModeView {
    let shown = if write_ok { target } else { readback };
    ModeView {
        shown,
        reboot_pending: shown != boot,
    }
}

/// Snap a slider value to the attribute's range and step.
///
/// The grid starts at `min`; the result never exceeds `max`. A non-positive
/// `step` (unreported by firmware) is treated as 1.
fn snap_to_step(value: f32, min: i32, max: i32, step: i32) -> i32 {
    let step = if step > 0 { step } else { 1 };
    let clamped = value.clamp(min as f32, max.max(min) as f32);
    let steps = ((clamped - min as f32) / step as f32).round() as i32;
    let mut snapped = min + steps * step;
    if snapped > max {
        snapped -= step;
    }
    snapped.max(min)
}

/// GPU switching capabilities exposed by asusd, plus the proxies to drive them.
///
/// `gpu_mux_mode` and `dgpu_disable` are independent firmware attributes: a
/// laptop may expose either, both, or neither. The available modes and whether
/// the control is shown at all are derived from which proxies are present.
struct GpuCaps {
    dgpu: Option<AsusArmouryProxy<'static>>,
    mux: Option<AsusArmouryProxy<'static>>,
}

impl GpuCaps {
    /// Collect the dgpu_disable / gpu_mux_mode proxies from asusd.
    async fn discover() -> Result<Self, Box<dyn std::error::Error>> {
        let attrs = find_iface_async::<AsusArmouryProxy>("xyz.ljones.AsusArmoury").await?;
        let mut caps = GpuCaps {
            dgpu: None,
            mux: None,
        };
        for attr in attrs {
            match attr.name().await {
                Ok(FirmwareAttribute::DgpuDisable) => caps.dgpu = Some(attr),
                Ok(FirmwareAttribute::GpuMuxMode) => caps.mux = Some(attr),
                Ok(_) => {}
                Err(e) => error!("setup_gpu_page: failed to read attribute name: {e:?}"),
            }
        }
        Ok(caps)
    }

    /// Whether any GPU switching is possible on this hardware.
    fn switchable(&self) -> bool {
        self.dgpu.is_some() || self.mux.is_some()
    }

    /// Modes this hardware supports, in display order.
    fn modes(&self) -> Vec<GpuMode> {
        let mut modes = Vec::new();
        if self.switchable() {
            modes.push(GpuMode::Hybrid);
        }
        if self.dgpu.is_some() {
            modes.push(GpuMode::Integrated);
        }
        if self.mux.is_some() {
            modes.push(GpuMode::Ultimate);
        }
        modes
    }

    /// Read the current hardware state and map it to a mode.
    async fn current_mode(&self) -> GpuMode {
        // Absent attributes report their inactive default so they never win below;
        // a read error is logged and falls back to the same default.
        let mux = match &self.mux {
            Some(a) => a.current_value().await.unwrap_or_else(|e| {
                error!("setup_gpu: failed to read gpu_mux_mode: {e:?}");
                1
            }),
            None => 1,
        };
        let dgpu = match &self.dgpu {
            Some(a) => a.current_value().await.unwrap_or_else(|e| {
                error!("setup_gpu: failed to read dgpu_disable: {e:?}");
                0
            }),
            None => 0,
        };
        if mux == 0 {
            GpuMode::Ultimate
        } else if dgpu == 1 {
            GpuMode::Integrated
        } else {
            GpuMode::Hybrid
        }
    }

    /// Apply a target mode, writing only the attributes that exist.
    async fn apply(&self, mode: GpuMode) -> zbus::Result<()> {
        let (dgpu_val, mux_val) = match mode {
            GpuMode::Integrated => (1, 1),
            GpuMode::Hybrid => (0, 1),
            GpuMode::Ultimate => (0, 0),
        };
        if let Some(attr) = &self.dgpu {
            attr.set_current_value(dgpu_val).await?;
        }
        if let Some(attr) = &self.mux {
            attr.set_current_value(mux_val).await?;
        }
        Ok(())
    }
}

/// Index of `mode` within `modes`, defaulting to 0 if not found.
fn index_of(modes: &[GpuMode], mode: GpuMode) -> i32 {
    modes.iter().position(|m| *m == mode).unwrap_or(0) as i32
}

fn set_selector_enabled(handle: &Weak<MainWindow>, enabled: bool) {
    handle
        .upgrade_in_event_loop(move |h| {
            h.global::<GPUPageData>().set_gpu_dropdown_enabled(enabled);
        })
        .unwrap_or_else(|e| error!("setup_gpu: failed to set dropdown state: {e:?}"));
}

/// Disable the selector, apply `mode`, toast the result, then refresh + re-enable.
fn set_gpu_mode(caps: Arc<GpuCaps>, handle: Weak<MainWindow>, boot: GpuMode, mode: GpuMode) {
    // Called from the slint callback on the event-loop thread, so disable the
    // selector synchronously here — a second selection can't slip in before the
    // write is queued.
    if let Some(h) = handle.upgrade() {
        h.global::<GPUPageData>().set_gpu_dropdown_enabled(false);
    }

    tokio::spawn(async move {
        let result = caps.apply(mode).await;
        let write_ok = result.is_ok();
        show_toast(
            SharedString::from("GPU mode change scheduled — reboot required for changes to apply."),
            SharedString::from("Failed to set GPU mode"),
            handle.clone(),
            result,
        );

        let view = mode_after_apply(boot, mode, write_ok, caps.current_mode().await);
        let new_index = index_of(&caps.modes(), view.shown);
        handle
            .upgrade_in_event_loop(move |h| {
                let global = h.global::<GPUPageData>();
                global.set_gpu_mode_index(new_index);
                global.set_gpu_reboot_pending(view.reboot_pending);
            })
            .unwrap_or_else(|e| error!("setup_gpu: failed to refresh mode: {e:?}"));

        set_selector_enabled(&handle, true);
    });
}

fn set_apu_mem(proxy: AsusArmouryProxy<'static>, handle: Weak<MainWindow>, value: i32) {
    let p = proxy.clone();
    let w = handle.clone();
    tokio::spawn(async move {
        let result = p.set_current_value(value).await;
        show_toast(
            SharedString::from(
                "Reserved GPU memory updated — reboot required for changes to apply.",
            ),
            SharedString::from("Failed to set reserved GPU memory"),
            w.clone(),
            result,
        );

        // Refresh the dropdown to show the (possibly unchanged) hardware state.
        let new_current = p.current_value().await.unwrap_or(value);
        let choices = p.possible_values().await.unwrap_or_default();
        let new_index = choices.iter().position(|v| *v == new_current).unwrap_or(0) as i32;
        w.upgrade_in_event_loop(move |h| {
            h.global::<GPUPageData>().set_apu_mem_index(new_index);
        })
        .unwrap_or_else(|e| error!("setup_gpu: failed to refresh apu_mem index: {e:?}"));
    });
}

fn apu_mem_val_to_label(value: i32) -> SharedString {
    if value == 0 {
        SharedString::from("AUTO")
    } else {
        SharedString::from(format!("{}G", value))
    }
}

// Populate GPU page choices and wire the `cb_set_gpu_mode` callback
pub fn setup_gpu_page(ui: &MainWindow) {
    let handle = ui.as_weak();
    ui.global::<GPUPageData>()
        .on_snap(snap_to_step);

    // Slider steps come from each attribute's ScalarIncrement, never hardcoded.
    let step_handle = ui.as_weak();
    tokio::spawn(async move {
        let Ok(attrs) = find_iface_async::<AsusArmouryProxy>("xyz.ljones.AsusArmoury").await else {
            return;
        };
        for attr in attrs {
            let Ok(name) = attr.name().await else {
                continue;
            };
            let step = attr.scalar_increment().await.unwrap_or(1);
            step_handle
                .upgrade_in_event_loop(move |h| {
                    let g = h.global::<GPUPageData>();
                    match name {
                        FirmwareAttribute::PptPl1Spl => g.set_ppt_pl1_step(step),
                        FirmwareAttribute::PptPl2Sppt => g.set_ppt_pl2_step(step),
                        FirmwareAttribute::PptPl3Fppt => g.set_ppt_pl3_step(step),
                        FirmwareAttribute::NvDynamicBoost => g.set_nv_boost_step(step),
                        FirmwareAttribute::NvTempTarget => g.set_nv_temp_step(step),
                        _ => {}
                    }
                })
                .ok();
        }
    });

    tokio::spawn(async move {
        let caps = match GpuCaps::discover().await {
            Ok(caps) => Arc::new(caps),
            Err(e) => {
                error!("setup_gpu_page: failed to get AsusArmoury proxies: {e:?}");
                return;
            }
        };

        let modes = caps.modes();
        let switchable = caps.switchable();
        let boot = caps.current_mode().await;
        let current_index = index_of(&modes, boot);
        let ids: Vec<i32> = modes.iter().map(|m| m.id()).collect();

        let caps_cb = caps.clone();
        let handle_cb = handle.clone();
        if let Err(e) = handle.upgrade_in_event_loop(move |handle| {
            let global = handle.global::<GPUPageData>();
            global.set_gpu_mode_ids(ids.as_slice().into());
            global.set_gpu_switchable(switchable);
            global.set_gpu_dropdown_enabled(switchable);
            global.set_gpu_mode_index(current_index);

            global.on_cb_set_gpu_mode(move |index| {
                let Some(mode) = modes.get(index as usize).copied() else {
                    return;
                };
                set_gpu_mode(caps_cb.clone(), handle_cb.clone(), boot, mode);
            });
        }) {
            error!("setup_gpu_page: upgrade_in_event_loop: {e:?}");
        }

        // --- APU mem ---
        let apu_mem_proxy: Option<AsusArmouryProxy<'static>> = async {
            let Ok(attrs) = find_iface_async::<AsusArmouryProxy>("xyz.ljones.AsusArmoury").await
            else {
                error!("setup_gpu: failed to find AsusArmoury proxies for apu_mem");
                return None;
            };
            for attr in attrs {
                match attr.name().await {
                    Ok(FirmwareAttribute::ApuMem) => return Some(attr),
                    Ok(_) => {}
                    Err(e) => error!("setup_gpu: failed to read attribute name: {e:?}"),
                }
            }
            None
        }
        .await;

        if let Some(proxy) = apu_mem_proxy {
            let possible = proxy.possible_values().await.unwrap_or_default();
            let current = proxy.current_value().await.unwrap_or(0);
            let apu_choices: Vec<SharedString> =
                possible.iter().map(|v| apu_mem_val_to_label(*v)).collect();
            let apu_index = possible.iter().position(|v| *v == current).unwrap_or(0) as i32;

            let proxy_cb = proxy.clone();
            let handle_cb = handle.clone();
            if let Err(e) = handle.upgrade_in_event_loop(move |h| {
                let global = h.global::<GPUPageData>();
                global.set_apu_mem_present(true);
                global.set_apu_mem_choices(apu_choices.as_slice().into());
                global.set_apu_mem_index(apu_index);
                let weak_handle = h.as_weak();
                global.on_cb_set_apu_mem(move |index| {
                    let Some(value) = possible.get(index as usize).copied() else {
                        return;
                    };
                    // Disable the dropdown while applying
                    weak_handle
                        .upgrade_in_event_loop(move |h| {
                            h.global::<GPUPageData>().set_apu_mem_index(index);
                        })
                        .unwrap_or_else(|e| {
                            error!("setup_gpu: failed to set apu_mem index: {e:?}")
                        });
                    set_apu_mem(proxy_cb.clone(), handle_cb.clone(), value);
                });
            }) {
                error!("setup_gpu: failed to wire apu_mem callback: {e:?}");
            }
        }

        // --- XG Mobile LED ---
        let xgm_results: Option<(XgmLedProxy<'static>, bool)> = async {
            let Ok(mut proxies) = find_iface_async::<XgmLedProxy>("xyz.ljones.XgmLed").await else {
                info!("setup_gpu: no XG Mobile LED interface");
                return None;
            };
            let xgm_proxy = proxies.pop()?;
            let enabled = xgm_proxy.xgm_led_enabled().await.unwrap_or(false);
            Some((xgm_proxy, enabled))
        }
        .await;
        if let Some((xgm_proxy, enabled)) = xgm_results {
            let handle_xgm = handle.clone();
            if let Err(e) = handle.upgrade_in_event_loop(move |h| {
                h.global::<GPUPageData>().set_has_xgm_led(true);
                h.global::<GPUPageData>().set_xgm_led_enabled(enabled);
            }) {
                error!("setup_gpu: failed to set XGM LED initial state: {e:?}");
            }

            // Wire callback
            let proxy_cb = xgm_proxy.clone();
            let handle_cb = handle_xgm.clone();
            if let Err(e) = handle_xgm.upgrade_in_event_loop(move |h| {
                h.global::<GPUPageData>()
                    .on_cb_set_xgm_led_enabled(move |checked| {
                        let p = proxy_cb.clone();
                        let w = handle_cb.clone();
                        tokio::spawn(async move {
                            let res = p.set_xgm_led_enabled(checked).await;
                            show_toast(
                                SharedString::from("XG Mobile LED updated"),
                                SharedString::from("Failed to set XG Mobile LED"),
                                w,
                                res,
                            );
                        });
                    });
            }) {
                error!("setup_gpu: failed to wire XGM LED callback: {e:?}");
            }

            // Listen for property changes
            let proxy_stream = xgm_proxy.clone();
            let handle_stream = handle.clone();
            tokio::spawn(async move {
                use futures_util::StreamExt;
                let mut stream = proxy_stream.receive_xgm_led_enabled_changed().await;
                while let Some(e) = stream.next().await {
                    if let Ok(enabled) = e.get().await {
                        handle_stream
                            .upgrade_in_event_loop(move |h| {
                                h.global::<GPUPageData>().set_xgm_led_enabled(enabled);
                            })
                            .ok();
                    }
                }
            });
        }

        // --- Disable nvidia-powerd on battery ---
        let platform_proxy = match zbus::Connection::system().await {
            Ok(conn) => match PlatformProxy::builder(&conn).build().await {
                Ok(p) => p,
                Err(e) => {
                    error!("setup_gpu: failed to create PlatformProxy: {e:?}");
                    return;
                }
            },
            Err(e) => {
                error!("setup_gpu: failed to connect to system bus: {e:?}");
                return;
            }
        };

        // Read initial value
        let initial = platform_proxy
            .disable_nvidia_powerd_on_battery()
            .await
            .unwrap_or(true);
        handle
            .upgrade_in_event_loop(move |h| {
                h.global::<GPUPageData>()
                    .set_disable_nvidia_powerd_on_battery(initial);
            })
            .ok();

        // Wire callback
        let proxy_cb = platform_proxy.clone();
        let handle_cb = handle.clone();
        handle
            .upgrade_in_event_loop(move |h| {
                h.global::<GPUPageData>()
                    .on_cb_disable_nvidia_powerd_on_battery(move |value| {
                        let p = proxy_cb.clone();
                        let w = handle_cb.clone();
                        tokio::spawn(async move {
                            let res = p.set_disable_nvidia_powerd_on_battery(value).await;
                            show_toast(
                                SharedString::from("Updated nvidia-powerd on-battery setting"),
                                SharedString::from(
                                    "Failed to update nvidia-powerd on-battery setting",
                                ),
                                w,
                                res,
                            );
                        });
                    });
            })
            .ok();

        // Listen for external changes
        let proxy_stream = platform_proxy.clone();
        let handle_stream = handle.clone();
        tokio::spawn(async move {
            use futures_util::StreamExt;
            let mut stream = proxy_stream
                .receive_disable_nvidia_powerd_on_battery_changed()
                .await;
            while let Some(e) = stream.next().await {
                if let Ok(value) = e.get().await {
                    handle_stream
                        .upgrade_in_event_loop(move |h| {
                            h.global::<GPUPageData>()
                                .set_disable_nvidia_powerd_on_battery(value);
                        })
                        .ok();
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_ids_are_distinct_and_stable() {
        assert_eq!(GpuMode::Integrated.id(), 0);
        assert_eq!(GpuMode::Hybrid.id(), 1);
        assert_eq!(GpuMode::Ultimate.id(), 2);
    }

    #[test]
    fn successful_change_shows_target_and_flags_reboot() {
        let v = mode_after_apply(GpuMode::Hybrid, GpuMode::Ultimate, true, GpuMode::Hybrid);
        assert_eq!(
            v,
            ModeView {
                shown: GpuMode::Ultimate,
                reboot_pending: true
            }
        );
    }

    #[test]
    fn switching_back_to_boot_mode_clears_pending() {
        let v = mode_after_apply(GpuMode::Hybrid, GpuMode::Hybrid, true, GpuMode::Ultimate);
        assert_eq!(
            v,
            ModeView {
                shown: GpuMode::Hybrid,
                reboot_pending: false
            }
        );
    }

    #[test]
    fn failed_write_falls_back_to_readback() {
        let v = mode_after_apply(GpuMode::Hybrid, GpuMode::Ultimate, false, GpuMode::Hybrid);
        assert_eq!(
            v,
            ModeView {
                shown: GpuMode::Hybrid,
                reboot_pending: false
            }
        );
    }

    #[test]
    fn snap_clamps_to_range() {
        assert_eq!(snap_to_step(3.0, 15, 80, 1), 15);
        assert_eq!(snap_to_step(200.0, 15, 80, 1), 80);
    }

    #[test]
    fn snap_rounds_to_step_grid_from_min() {
        assert_eq!(snap_to_step(17.4, 15, 80, 5), 15);
        assert_eq!(snap_to_step(17.6, 15, 80, 5), 20);
    }

    #[test]
    fn snap_never_exceeds_max_when_grid_misses_it() {
        // grid 5,10,15 with max 17: 17 rounds to 15 not 20
        assert_eq!(snap_to_step(17.0, 5, 17, 5), 15);
    }

    #[test]
    fn snap_treats_missing_step_as_one() {
        assert_eq!(snap_to_step(42.4, 0, 100, 0), 42);
        assert_eq!(snap_to_step(42.6, 0, 100, -1), 43);
    }
}
