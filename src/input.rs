use terrabox::{Key, KeyEvent, Modifiers};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(i64, i64),
    Accept,
    NewWorld,
    Pause,
    Menu,
    Quit,
}
pub fn action(event: KeyEvent) -> Option<Action> {
    if event.modifiers.contains(Modifiers::CTRL) {
        return matches!(event.key, Key::Char('c' | 'C' | 'q' | 'Q')).then_some(Action::Quit);
    }
    if event.modifiers.contains(Modifiers::ALT) {
        return None;
    }
    match event.key {
        Key::Up | Key::Char('w' | 'W') => Some(Action::Move(0, -1)),
        Key::Down | Key::Char('s' | 'S') => Some(Action::Move(0, 1)),
        Key::Left | Key::Char('a' | 'A') => Some(Action::Move(-1, 0)),
        Key::Right | Key::Char('d' | 'D') => Some(Action::Move(1, 0)),
        Key::Enter => Some(Action::Accept),
        Key::Char('n' | 'N') => Some(Action::NewWorld),
        Key::Char('p' | 'P' | ' ') => Some(Action::Pause),
        Key::Char('m' | 'M') => Some(Action::Menu),
        Key::Escape | Key::Char('q' | 'Q') => Some(Action::Quit),
        _ => None,
    }
}
