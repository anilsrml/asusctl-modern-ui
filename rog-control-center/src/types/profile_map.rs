//! Pure mapping between `PlatformProfile` enum values (as sent over D-Bus)
//! and dropdown positions, for any order/subset of `PlatformProfileChoices`.

/// Sorted, de-duplicated enum values that have a UI label (`0..label_count`).
pub fn dropdown_indexes(choices: &[i32], label_count: usize) -> Vec<i32> {
    let mut v: Vec<i32> = choices
        .iter()
        .copied()
        .filter(|c| *c >= 0 && (*c as usize) < label_count)
        .collect();
    v.sort_unstable();
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
                3, 2, 0, 1,
            ],
            4,
        );
        assert_eq!(
            idx,
            vec![
                0, 1, 2, 3
            ]
        );
        assert_eq!(position_of(&idx, 1), Some(1));
        assert_eq!(value_at(&idx, 3), Some(3));
    }

    #[test]
    fn subset_choices_use_position_not_value() {
        let idx = dropdown_indexes(&[3, 0, 1], 4);
        assert_eq!(idx, vec![0, 1, 3]);
        assert_eq!(position_of(&idx, 3), Some(2));
        assert_eq!(value_at(&idx, 2), Some(3));
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
