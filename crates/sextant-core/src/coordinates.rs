//! slot geometry for magnetically connected components.
//!
//! a component sits in an integer grid `slot` and faces an `orientation` of 0,
//! 90, 180, or 270 degrees. its single male port connects toward its parent;
//! its female ports expose the slots where children attach. "wide" components
//! (a double-width base with five female ports) use a separate table.

pub type Slot = (i32, i32);
pub type Orientation = i32;

/// slot occupied by the male (upstream) port for a component at `slot`.
pub fn calculate_male_slot(slot: Slot, orientation: Orientation) -> Slot {
    let (x, y) = slot;
    match orientation {
        0 => (x, y + 1),
        90 => (x + 1, y),
        180 => (x, y - 1),
        270 => (x - 1, y),
        _ => slot,
    }
}

/// the three female (downstream) port slots for a square component.
pub fn calculate_female_slots(slot: Slot, orientation: Orientation) -> [Slot; 3] {
    let (x, y) = slot;
    match orientation {
        0 => [(x + 1, y), (x, y - 1), (x - 1, y)],
        180 => [(x - 1, y), (x, y + 1), (x + 1, y)],
        90 => [(x, y - 1), (x - 1, y), (x, y + 1)],
        270 => [(x, y + 1), (x + 1, y), (x, y - 1)],
        _ => [slot, slot, slot],
    }
}

/// the five female port slots for a wide (double-width) component.
///
/// the 180-degree third slot is `(x - 1, y + 1)` here. upstream had a typo,
/// `(x - 1, x + 1)`, which only matched the reference tests because they were
/// taken at the origin where `x == y`. the value is the 180-degree rotation of
/// the 0-degree third slot `(x + 1, y - 1)`, so the corrected form is used.
pub fn calculate_female_slots_wide(slot: Slot, orientation: Orientation) -> [Slot; 5] {
    let (x, y) = slot;
    match orientation {
        0 => [
            (x + 1, y + 1),
            (x + 2, y),
            (x + 1, y - 1),
            (x, y - 1),
            (x - 1, y),
        ],
        180 => [
            (x - 1, y - 1),
            (x - 2, y),
            (x - 1, y + 1),
            (x, y + 1),
            (x + 1, y),
        ],
        90 => [
            (x + 1, y - 1),
            (x, y - 2),
            (x - 1, y - 1),
            (x - 1, y),
            (x, y + 1),
        ],
        270 => [
            (x - 1, y + 1),
            (x, y + 2),
            (x + 1, y + 1),
            (x + 1, y),
            (x, y - 1),
        ],
        _ => [slot, slot, slot, slot, slot],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_male_port() {
        assert_eq!((0, 1), calculate_male_slot((0, 0), 0));
        assert_eq!((0, -1), calculate_male_slot((0, 0), 180));
        assert_eq!((1, 0), calculate_male_slot((0, 0), 90));
        assert_eq!((-1, 0), calculate_male_slot((0, 0), 270));
    }

    #[test]
    fn narrow_female_ports() {
        assert_eq!(
            [(1, 0), (0, -1), (-1, 0)],
            calculate_female_slots((0, 0), 0)
        );
        assert_eq!(
            [(-1, 0), (0, 1), (1, 0)],
            calculate_female_slots((0, 0), 180)
        );
        assert_eq!(
            [(0, -1), (-1, 0), (0, 1)],
            calculate_female_slots((0, 0), 90)
        );
        assert_eq!(
            [(0, 1), (1, 0), (0, -1)],
            calculate_female_slots((0, 0), 270)
        );
    }

    #[test]
    fn wide_male_port() {
        assert_eq!((0, 1), calculate_male_slot((0, 0), 0));
        assert_eq!((0, -1), calculate_male_slot((0, 0), 180));
        assert_eq!((1, 0), calculate_male_slot((0, 0), 90));
        assert_eq!((-1, 0), calculate_male_slot((0, 0), 270));
    }

    #[test]
    fn wide_female_ports() {
        assert_eq!(
            [(1, 1), (2, 0), (1, -1), (0, -1), (-1, 0)],
            calculate_female_slots_wide((0, 0), 0)
        );
        assert_eq!(
            [(-1, -1), (-2, 0), (-1, 1), (0, 1), (1, 0)],
            calculate_female_slots_wide((0, 0), 180)
        );
        assert_eq!(
            [(1, -1), (0, -2), (-1, -1), (-1, 0), (0, 1)],
            calculate_female_slots_wide((0, 0), 90)
        );
        assert_eq!(
            [(-1, 1), (0, 2), (1, 1), (1, 0), (0, -1)],
            calculate_female_slots_wide((0, 0), 270)
        );
    }

    /// off-origin regression that distinguishes the corrected wide-180 third
    /// slot `(x - 1, y + 1)` from the upstream `(x - 1, x + 1)` typo.
    #[test]
    fn wide_female_180_off_origin() {
        assert_eq!((2, 2), calculate_female_slots_wide((3, 1), 180)[2]);
    }
}
