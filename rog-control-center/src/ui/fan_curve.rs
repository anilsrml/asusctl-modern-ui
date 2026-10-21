//! Pure validation for fan-curve editing: temperatures and PWM values must
//! stay within the hardware range and never decrease between neighbours.

pub const TEMP_MAX: f32 = 100.0;
pub const PWM_MAX: f32 = 255.0;

/// Clamp a proposed `(temp, pwm)` for point `idx` so that the curve stays
/// monotonic non-decreasing in both axes, and within `0..=TEMP_MAX` and
/// `0..=PWM_MAX`. NaN input keeps the current point. Values are rounded to whole numbers as the firmware takes `u8`.
pub fn clamp_point(points: &[(f32, f32)], idx: usize, proposed: (f32, f32)) -> (f32, f32) {
    let Some(&current) = points.get(idx) else {
        return proposed;
    };
    if proposed.0.is_nan() || proposed.1.is_nan() {
        return current;
    }
    let (lo_x, lo_y) = if idx > 0 { points[idx - 1] } else { (0.0, 0.0) };
    let (hi_x, hi_y) = points.get(idx + 1).copied().unwrap_or((TEMP_MAX, PWM_MAX));
    // Hardware data may already be non-monotonic; never let `clamp` see min > max.
    (
        proposed.0.round().clamp(lo_x, hi_x.max(lo_x)),
        proposed.1.round().clamp(lo_y, hi_y.max(lo_y)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURVE: [(f32, f32); 4] = [
        (10.0, 10.0),
        (40.0, 30.0),
        (60.0, 60.0),
        (90.0, 100.0),
    ];

    #[test]
    fn middle_point_cannot_pass_next_neighbour() {
        assert_eq!(clamp_point(&CURVE, 1, (75.0, 40.0)), (60.0, 40.0));
        assert_eq!(clamp_point(&CURVE, 1, (30.0, 80.0)), (30.0, 60.0));
    }

    #[test]
    fn middle_point_cannot_pass_previous_neighbour() {
        assert_eq!(clamp_point(&CURVE, 2, (5.0, 5.0)), (40.0, 30.0));
    }

    #[test]
    fn first_and_last_use_global_bounds() {
        assert_eq!(clamp_point(&CURVE, 0, (-5.0, -1.0)), (0.0, 0.0));
        assert_eq!(clamp_point(&CURVE, 3, (150.0, 400.0)), (100.0, 255.0));
        assert_eq!(clamp_point(&CURVE, 0, (50.0, 10.0)), (40.0, 10.0));
    }

    #[test]
    fn valid_values_are_rounded_not_moved() {
        assert_eq!(clamp_point(&CURVE, 1, (41.4, 30.6)), (41.0, 31.0));
    }

    #[test]
    fn bad_index_or_nan_is_safe() {
        assert_eq!(clamp_point(&CURVE, 9, (5.0, 5.0)), (5.0, 5.0));
        assert_eq!(clamp_point(&CURVE, 1, (f32::NAN, 20.0)), (40.0, 30.0));
    }
}
