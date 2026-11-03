//! keyboard mapping for the simulator, kept free of the render loop so it can be
//! unit tested. digit keys select a component, page up/down cycle the selection,
//! tab rotates the layout, arrows and space drive the selected component's input
//! injectors, and q / escape quit.

use macroquad::input::KeyCode;

/// an action produced by a key event, applied by the render loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimAction {
    Quit,
    ToggleHelp,
    Rotate,
    SelectPrev,
    SelectNext,
    Select(i64),
    SlideUp,
    SlideDown,
    RotateLeft,
    RotateRight,
    Press,
    Release,
}

/// map a key event to a simulator action. `pressed` is true for key-down and
/// false for key-up, matching the upstream split between the two.
pub fn key_action(key: KeyCode, pressed: bool) -> Option<SimAction> {
    use KeyCode::*;
    let action = match (key, pressed) {
        (Q, true) | (Escape, true) => SimAction::Quit,
        // the `?` key (shift+slash) toggles the keybinding overlay.
        (Slash, true) => SimAction::ToggleHelp,
        (Tab, false) => SimAction::Rotate,
        (PageUp, false) => SimAction::SelectPrev,
        (PageDown, false) => SimAction::SelectNext,
        (Up, true) => SimAction::SlideUp,
        (Down, true) => SimAction::SlideDown,
        (Left, true) => SimAction::RotateLeft,
        (Right, true) => SimAction::RotateRight,
        (Space, true) => SimAction::Press,
        (Space, false) => SimAction::Release,
        (Key0, true) => SimAction::Select(0),
        (Key1, true) => SimAction::Select(1),
        (Key2, true) => SimAction::Select(2),
        (Key3, true) => SimAction::Select(3),
        (Key4, true) => SimAction::Select(4),
        (Key5, true) => SimAction::Select(5),
        (Key6, true) => SimAction::Select(6),
        (Key7, true) => SimAction::Select(7),
        (Key8, true) => SimAction::Select(8),
        (Key9, true) => SimAction::Select(9),
        _ => return None,
    };
    Some(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_keys_to_actions() {
        assert_eq!(Some(SimAction::Quit), key_action(KeyCode::Q, true));
        assert_eq!(Some(SimAction::Quit), key_action(KeyCode::Escape, true));
        assert_eq!(
            Some(SimAction::ToggleHelp),
            key_action(KeyCode::Slash, true)
        );
        assert_eq!(Some(SimAction::Rotate), key_action(KeyCode::Tab, false));
        assert_eq!(Some(SimAction::Select(2)), key_action(KeyCode::Key2, true));
        assert_eq!(Some(SimAction::SlideUp), key_action(KeyCode::Up, true));
        assert_eq!(Some(SimAction::Press), key_action(KeyCode::Space, true));
        assert_eq!(Some(SimAction::Release), key_action(KeyCode::Space, false));
        // tab only acts on key-up; a down event is ignored.
        assert_eq!(None, key_action(KeyCode::Tab, true));
    }
}
