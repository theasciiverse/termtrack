//! Keyboard input from the terminal.
//!
//! A reader thread turns the terminal's key events into two things:
//! which keys are held now (for walking, turning and jumping in a level),
//! and a queue of presses (for menus and one-shot keys such as the map).
//!
//! Terminals that speak the kitty keyboard protocol (kitty, WezTerm, foot,
//! Ghostty, Alacritty, ...) and the Windows console report key releases, so
//! a key is held from its press to its release. Most others (GNOME Terminal,
//! Konsole, xterm, tmux, macOS Terminal) send only presses and the system's
//! key repeat. There a key counts as held from its press until its repeats
//! stop: long enough after the first press to cover the repeat delay (learned
//! from the first repeat seen), and briefly after each repeat. A key that was
//! already repeating stays held while another key is pressed, since those
//! terminals repeat only the last key pressed.

use crossterm::event::{
    read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Keys the game cares about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    W,
    A,
    S,
    D,
    Space,
    Up,
    Down,
    Left,
    Right,
    Enter,
    E,
    M,
    R,
    L,
    Escape,
    /// Ctrl-C, or Q: leave the game at once.
    Quit,
}

fn key_of(event: &KeyEvent) -> Option<Key> {
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        return match event.code {
            KeyCode::Char('c') | KeyCode::Char('C') => Some(Key::Quit),
            _ => None,
        };
    }
    Some(match event.code {
        KeyCode::Char('w') | KeyCode::Char('W') => Key::W,
        KeyCode::Char('a') | KeyCode::Char('A') => Key::A,
        KeyCode::Char('s') | KeyCode::Char('S') => Key::S,
        KeyCode::Char('d') | KeyCode::Char('D') => Key::D,
        KeyCode::Char(' ') => Key::Space,
        KeyCode::Char('e') | KeyCode::Char('E') => Key::E,
        KeyCode::Char('m') | KeyCode::Char('M') => Key::M,
        KeyCode::Char('r') | KeyCode::Char('R') => Key::R,
        KeyCode::Char('l') | KeyCode::Char('L') => Key::L,
        KeyCode::Char('q') | KeyCode::Char('Q') => Key::Quit,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        _ => return None,
    })
}

/// Before the first repeat of a key, how long it counts as held.
const FIRST_HOLD: Duration = Duration::from_millis(550);
/// After a repeat, how long until the key counts as let go.
const REPEAT_HOLD: Duration = Duration::from_millis(110);

#[derive(Clone, Copy)]
struct Hold {
    first: Instant,
    /// the second event of this hold: the first repeat, or a second tap
    second: Option<Instant>,
    until: Instant,
    repeating: bool,
}

#[derive(Default)]
struct State {
    /// held keys; for terminals without releases, with their deadlines
    held: HashMap<Key, Hold>,
    presses: VecDeque<Key>,
}

#[derive(Clone)]
pub struct Input {
    state: Arc<Mutex<State>>,
    releases: bool,
}

impl Input {
    /// Asks the terminal for key releases and starts the reader thread.
    /// Call after raw mode is on.
    pub fn start() -> Input {
        let releases = if cfg!(windows) {
            true // the Windows console reports releases on its own
        } else {
            matches!(crossterm::terminal::supports_keyboard_enhancement(), Ok(true))
        };
        if releases && !cfg!(windows) {
            let _ = crossterm::execute!(
                std::io::stdout(),
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
            );
        }
        log::info!(
            "input: {}",
            if releases { "the terminal reports key releases" } else { "no key releases; holds follow key repeat" }
        );
        let input = Input { state: Arc::new(Mutex::new(State::default())), releases };
        let reader = input.clone();
        thread::spawn(move || reader.read_loop());
        input
    }

    fn read_loop(&self) {
        // learned key repeat delay, for terminals without releases
        let mut first_hold = FIRST_HOLD;
        loop {
            let event = match read() {
                Ok(event) => event,
                Err(err) => {
                    log::error!("reading the terminal failed: {err}");
                    return;
                }
            };
            log::debug!("event {:?}", event);
            let Event::Key(key_event) = event else { continue };
            let Some(key) = key_of(&key_event) else { continue };
            let now = Instant::now();
            let mut state = self.state.lock().unwrap();
            match key_event.kind {
                KeyEventKind::Release => {
                    state.held.remove(&key);
                }
                KeyEventKind::Press | KeyEventKind::Repeat => {
                    state.presses.push_back(key);
                    if state.presses.len() > 32 {
                        state.presses.pop_front();
                    }
                    if self.releases {
                        state.held.insert(key, Hold { first: now, second: None, until: now, repeating: true });
                        continue;
                    }
                    let hold = match state.held.get(&key).copied() {
                        Some(hold) if hold.until >= now => {
                            // a third event soon after the second shows a
                            // repeat stream (not two taps): learn the delay
                            if let (Some(second), false) = (hold.second, hold.repeating) {
                                let delay = second - hold.first;
                                if now - second < Duration::from_millis(150)
                                    && delay > Duration::from_millis(150)
                                    && delay < Duration::from_millis(1000)
                                {
                                    first_hold = delay + Duration::from_millis(80);
                                    log::debug!("key repeat delay {:?}", delay);
                                }
                            }
                            Hold {
                                first: hold.first,
                                second: hold.second.or(Some(now)),
                                until: now + REPEAT_HOLD,
                                repeating: hold.second.is_some(),
                            }
                        }
                        _ => Hold { first: now, second: None, until: now + first_hold, repeating: false },
                    };
                    // the terminal repeats only the last key pressed, so a key
                    // that was repeating is taken to be still down
                    for (other, h) in state.held.iter_mut() {
                        if *other != key && h.repeating && h.until >= now {
                            h.until = h.until.max(now + first_hold);
                        }
                    }
                    state.held.insert(key, hold);
                }
            }
        }
    }

    /// Whether a key is down now.
    pub fn held(&self, key: Key) -> bool {
        let mut state = self.state.lock().unwrap();
        if self.releases {
            return state.held.contains_key(&key);
        }
        let now = Instant::now();
        state.held.retain(|_, h| h.until >= now);
        state.held.contains_key(&key)
    }

    /// The keys down now, for the log.
    pub fn held_keys(&self) -> Vec<Key> {
        let all = [
            Key::W, Key::A, Key::S, Key::D, Key::Space, Key::Up, Key::Down, Key::Left, Key::Right,
        ];
        all.into_iter().filter(|k| self.held(*k)).collect()
    }

    /// The next key pressed (or repeated), if any.
    pub fn next_press(&self) -> Option<Key> {
        self.state.lock().unwrap().presses.pop_front()
    }

    /// Waits for the next key pressed.
    pub fn wait_press(&self) -> Key {
        loop {
            if let Some(key) = self.next_press() {
                return key;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Forgets presses not yet handled, e.g. when a screen changes.
    pub fn clear_presses(&self) {
        self.state.lock().unwrap().presses.clear();
    }
}
