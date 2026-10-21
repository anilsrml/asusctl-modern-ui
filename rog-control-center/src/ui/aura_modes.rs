//! Pure helpers that turn what asusd reports (supported basic modes, current
//! mode, brightness) into the indexes the Aura page shows.

/// Raw mode number of Pulse as defined by `AuraModeNum`.
const PULSE: i32 = 10;
/// Pulse comes back as 9 from some `AllModeData` replies (enum index instead of
/// discriminant). 9 is not a real effect, so it always means Pulse.
const PULSE_ALIAS: i32 = 9;
/// Highest raw mode number in `AuraModeNum` (Flash).
const MAX_MODE: i32 = 12;
/// Highest brightness level asusd accepts.
const MAX_BRIGHTNESS: i32 = 3;

/// Canonical raw mode number, or `None` if it is not a known effect.
pub fn canonical_mode(raw: i32) -> Option<i32> {
    match raw {
        PULSE_ALIAS => Some(PULSE),
        0..=MAX_MODE => Some(raw),
        _ => None,
    }
}

/// Modes to show as chips: only what asusd reports, in the order reported,
/// with the 9/10 mismatch resolved, unknown values dropped and no duplicates.
pub fn mode_chips(supported: &[i32]) -> Vec<i32> {
    let mut out = Vec::new();
    for raw in supported.iter().filter_map(|r| canonical_mode(*r)) {
        if !out.contains(&raw) {
            out.push(raw);
        }
    }
    out
}

/// Position of the current mode within the chips, if it is one of them.
pub fn mode_index(chips: &[i32], current: i32) -> Option<usize> {
    let current = canonical_mode(current)?;
    chips.iter().position(|c| *c == current)
}

/// Segment to highlight for a brightness level, clamped to the 0..=3 range.
pub fn brightness_index(level: i32) -> i32 {
    level.clamp(0, MAX_BRIGHTNESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chips_follow_supported_modes() {
        assert_eq!(
            mode_chips(&[
                0, 1, 2, 3, 10
            ]),
            vec![
                0, 1, 2, 3, 10
            ]
        );
    }

    #[test]
    fn chips_resolve_pulse_reported_as_9() {
        assert_eq!(
            mode_chips(&[
                0, 1, 2, 3, 9
            ]),
            vec![
                0, 1, 2, 3, 10
            ]
        );
    }

    #[test]
    fn chips_drop_unknown_and_duplicates() {
        assert_eq!(
            mode_chips(&[
                0, 99, -1, 9, 10, 0
            ]),
            vec![0, 10]
        );
    }

    #[test]
    fn chips_empty_when_nothing_supported() {
        assert!(mode_chips(&[]).is_empty());
    }

    #[test]
    fn index_finds_pulse_under_either_number() {
        let chips = mode_chips(&[
            0, 1, 2, 3, 10,
        ]);
        assert_eq!(mode_index(&chips, 10), Some(4));
        assert_eq!(mode_index(&chips, 9), Some(4));
    }

    #[test]
    fn index_is_none_for_unsupported_mode() {
        assert_eq!(mode_index(&[0, 1], 5), None);
        assert_eq!(mode_index(&[0, 1], 99), None);
    }

    #[test]
    fn brightness_maps_to_segment_and_clamps() {
        assert_eq!(brightness_index(0), 0);
        assert_eq!(brightness_index(3), 3);
        assert_eq!(brightness_index(7), 3);
        assert_eq!(brightness_index(-2), 0);
    }
}
