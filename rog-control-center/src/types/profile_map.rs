//! Pure mapping between `PlatformProfile` enum values (as sent over D-Bus)
//! and dropdown positions, for any order/subset of `PlatformProfileChoices`.

/// Display rank of a profile enum value: quietest first, most powerful last.
/// Values without a known rank keep enum order after the known ones.
fn power_rank(value: i32) -> i32 {
    match value {
        3 => 0, // LowPower
        2 => 1, // Quiet
        0 => 2, // Balanced
        1 => 3, // Performance
        other => 4 + other,
    }
}

/// De-duplicated enum values that have a UI label (`0..label_count`),
/// ordered from the quietest to the most powerful profile.
pub fn dropdown_indexes(choices: &[i32], label_count: usize) -> Vec<i32> {
    let mut v: Vec<i32> = choices
        .iter()
        .copied()
        .filter(|c| *c >= 0 && (*c as usize) < label_count)
        .collect();
    v.sort_unstable_by_key(|c| power_rank(*c));
    v.dedup();
    v
}

/// Dropdown position showing `value`, or `None` if it is not offered.
pub fn position_of(indexes: &[i32], value: i32) -> Option<usize> {
    indexes.iter().position(|i| *i == value)
}

/// Enum value to send for a dropdown position, or `None` if out of range.
pub fn value_at(indexes: &[i32], position: i32) -> Option<i32> {
    usize::try_from(position)
        .ok()
        .and_then(|p| indexes.get(p).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unordered_choices_map_both_ways() {
        let idx = dropdown_indexes(
            &[
                1, 0, 3, 2,
            ],
            4,
        );
        assert_eq!(position_of(&idx, 1), Some(3));
        assert_eq!(value_at(&idx, 0), Some(3));
    }

    #[test]
    fn subset_choices_use_position_not_value() {
        let idx = dropdown_indexes(&[1, 3, 0], 4);
        assert_eq!(idx, vec![3, 0, 1]);
        assert_eq!(position_of(&idx, 3), Some(0));
        assert_eq!(value_at(&idx, 2), Some(1));
    }

    #[test]
    fn choices_are_ordered_from_quietest_to_most_powerful() {
        // LowPower=3, Quiet=2, Balanced=0, Performance=1
        assert_eq!(
            dropdown_indexes(
                &[
                    0, 1, 2, 3
                ],
                4
            ),
            vec![
                3, 2, 0, 1
            ]
        );
        assert_eq!(dropdown_indexes(&[1, 2, 0], 4), vec![2, 0, 1]);
    }

    #[test]
    fn unavailable_value_has_no_position() {
        let idx = dropdown_indexes(&[0, 1, 3], 4);
        assert_eq!(position_of(&idx, 4), None);
        assert_eq!(position_of(&idx, 2), None);
        assert_eq!(value_at(&idx, -1), None);
        assert_eq!(value_at(&idx, 9), None);
    }
}
