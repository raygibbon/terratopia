use terrabox::{Key, KeyEvent, Modifiers};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Move(i64, i64),
    Accept,
    NewWorld,
    Pause,
    Scan,
    Menu,
    Quit,
    QuitWithoutSave,
    Interact,
    Build,
    UseOxygen,
    Inventory,
    Craft,
    Help,
    Back,
    Discard,
    Save,
    Load,
    Rest,
    Attack,
}
pub fn action(event: KeyEvent) -> Option<Action> {
    if event.modifiers.contains(Modifiers::CTRL) {
        return match event.key {
            Key::Char('q' | 'Q') => Some(Action::QuitWithoutSave),
            Key::Char('c' | 'C') => Some(Action::Quit),
            _ => None,
        };
    }
    if event.modifiers.contains(Modifiers::ALT) {
        return None;
    }
    match event.key {
        Key::Up | Key::Char('w' | 'W') => Some(Action::Move(0, -1)),
        Key::Down | Key::Char('s' | 'S') => Some(Action::Move(0, 1)),
        Key::Left | Key::Char('a' | 'A') => Some(Action::Move(-1, 0)),
        Key::Right | Key::Char('d' | 'D') => Some(Action::Move(1, 0)),
        Key::Char('r' | 'R') => Some(Action::Scan),
        Key::Char('e' | 'E') => Some(Action::Interact),
        Key::Char('b' | 'B') => Some(Action::Build),
        Key::Char('o' | 'O') => Some(Action::UseOxygen),
        Key::Char('i' | 'I') => Some(Action::Inventory),
        Key::Char('c' | 'C') => Some(Action::Craft),
        Key::Char('h' | 'H' | '?') | Key::Function(1) => Some(Action::Help),
        Key::Backspace => Some(Action::Back),
        Key::Char('x' | 'X') => Some(Action::Discard),
        Key::Function(5) => Some(Action::Save),
        Key::Function(9) | Key::Char('l' | 'L') => Some(Action::Load),
        Key::Char('z' | 'Z') => Some(Action::Rest),
        Key::Char('f' | 'F') => Some(Action::Attack),
        Key::Enter => Some(Action::Accept),
        Key::Char('n' | 'N') => Some(Action::NewWorld),
        Key::Char('p' | 'P' | ' ') => Some(Action::Pause),
        Key::Char('m' | 'M') => Some(Action::Menu),
        Key::Escape | Key::Char('q' | 'Q') => Some(Action::Quit),
        _ => None,
    }
}
