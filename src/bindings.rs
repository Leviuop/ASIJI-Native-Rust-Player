use anyhow::{Result, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Pause,
    Backward,
    Forward,
    VolumeUp,
    VolumeDown,
    Mute,
    Color,
    Repeat,
    Mode,
    Next,
    Previous,
    Menu,
    Quit,
}

const DEFAULTS: &[(&str, Control, &[&str])] = &[
    ("pause", Control::Pause, &["space"]),
    ("seek_backward", Control::Backward, &["left", "a"]),
    ("seek_forward", Control::Forward, &["right", "d"]),
    ("volume_up", Control::VolumeUp, &["up", "+", "="]),
    ("volume_down", Control::VolumeDown, &["down", "-"]),
    ("mute", Control::Mute, &["m"]),
    ("color", Control::Color, &["c"]),
    ("repeat", Control::Repeat, &["r"]),
    ("mode", Control::Mode, &["h"]),
    ("next", Control::Next, &["n"]),
    ("previous", Control::Previous, &["p"]),
    ("menu", Control::Menu, &["q", "esc"]),
    ("quit", Control::Quit, &["x", "ctrl+c"]),
];

type Key = (KeyCode, KeyModifiers);

#[derive(Clone)]
pub struct Bindings {
    pub names: BTreeMap<String, Vec<String>>,
    keys: HashMap<Key, Control>,
}

impl Default for Bindings {
    fn default() -> Self {
        Self::new(BTreeMap::new()).expect("valid default bindings")
    }
}

fn normalized(mut key: Key) -> Key {
    key.1 &= KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT;
    if let KeyCode::Char(c) = key.0 {
        if c.is_ascii_uppercase() {
            key.1.insert(KeyModifiers::SHIFT);
        }
        if !c.is_alphabetic() {
            key.1.remove(KeyModifiers::SHIFT);
        }
        key.0 = KeyCode::Char(c.to_ascii_lowercase());
    }
    key
}

fn parse_key(text: &str) -> Result<Key> {
    let lowered = text.to_lowercase();
    let mut rest = lowered.as_str();
    let mut modifiers = KeyModifiers::NONE;
    loop {
        let found = [
            ("ctrl+", KeyModifiers::CONTROL),
            ("alt+", KeyModifiers::ALT),
            ("shift+", KeyModifiers::SHIFT),
        ]
        .into_iter()
        .find(|(prefix, _)| rest.starts_with(prefix));
        let Some((prefix, modifier)) = found else {
            break;
        };
        anyhow::ensure!(
            !modifiers.contains(modifier),
            "Повтор модификатора в {text}"
        );
        modifiers.insert(modifier);
        rest = &rest[prefix.len()..];
    }
    let code = match rest {
        "space" => KeyCode::Char(' '),
        "enter" => KeyCode::Enter,
        "esc" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "insert" => KeyCode::Insert,
        "delete" => KeyCode::Delete,
        _ if rest.chars().count() == 1 && !rest.chars().next().unwrap().is_control() => {
            KeyCode::Char(rest.chars().next().unwrap())
        }
        _ => {
            let function = rest
                .strip_prefix('f')
                .and_then(|s| s.parse::<u8>().ok())
                .filter(|n| (1..=12).contains(n));
            if let Some(n) = function {
                KeyCode::F(n)
            } else {
                bail!("Неизвестная клавиша: {text}");
            }
        }
    };
    if code == KeyCode::Tab && modifiers.contains(KeyModifiers::SHIFT) {
        return Ok((KeyCode::BackTab, modifiers));
    }
    Ok(normalized((code, modifiers)))
}

impl Bindings {
    pub fn new(overrides: BTreeMap<String, Vec<String>>) -> Result<Self> {
        let mut names: BTreeMap<String, Vec<String>> = DEFAULTS
            .iter()
            .map(|(name, _, keys)| {
                (
                    (*name).into(),
                    keys.iter().map(|key| (*key).into()).collect(),
                )
            })
            .collect();
        for (name, keys) in overrides {
            anyhow::ensure!(
                names.contains_key(&name),
                "Неизвестное действие bindings.{name}"
            );
            names.insert(name, keys);
        }
        let mut keys = HashMap::new();
        for (name, control, _) in DEFAULTS {
            for key in &names[*name] {
                let parsed = parse_key(key)?;
                anyhow::ensure!(
                    parsed != (KeyCode::Char('c'), KeyModifiers::CONTROL)
                        || *control == Control::Quit,
                    "Ctrl+C зарезервирован для выхода"
                );
                if let Some(previous) = keys.insert(parsed, *control) {
                    anyhow::ensure!(
                        previous == *control,
                        "Конфликт клавиши {key}: {previous:?} и {control:?}"
                    );
                }
            }
        }
        if !names["quit"]
            .iter()
            .any(|s| parse_key(s).ok() == Some((KeyCode::Char('c'), KeyModifiers::CONTROL)))
        {
            names.get_mut("quit").unwrap().push("ctrl+c".into());
        }
        keys.insert((KeyCode::Char('c'), KeyModifiers::CONTROL), Control::Quit);
        Ok(Self { names, keys })
    }

    pub fn action(&self, event: KeyEvent) -> Option<Control> {
        let mut key = normalized((event.code, event.modifiers));
        if let Some(action) = self.keys.get(&key) {
            return Some(*action);
        }
        // Preserve case-insensitive defaults unless a shifted key was explicitly bound.
        if matches!(key.0, KeyCode::Char(_)) && key.1.contains(KeyModifiers::SHIFT) {
            key.1.remove(KeyModifiers::SHIFT);
            return self.keys.get(&key).copied();
        }
        None
    }

    pub fn hint(&self, action: &str) -> &str {
        self.names
            .get(action)
            .and_then(|keys| keys.first())
            .map(String::as_str)
            .unwrap_or("—")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conflicts_and_unknowns_fail() {
        for (action, key) in [
            ("pause", "q"),
            ("pause", "ctrl+c"),
            ("paus", "space"),
            ("pause", "banana"),
        ] {
            assert!(Bindings::new(BTreeMap::from([(action.into(), vec![key.into()])])).is_err());
        }
    }
    #[test]
    fn overrides_replace_defaults_and_keep_exit() {
        let bindings = Bindings::new(BTreeMap::from([
            ("pause".into(), vec!["f2".into()]),
            ("quit".into(), vec![]),
        ]))
        .unwrap();
        assert_eq!(
            bindings.action(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE)),
            Some(Control::Pause)
        );
        assert_eq!(
            bindings.action(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            bindings.action(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Control::Quit)
        );
    }
    #[test]
    fn character_case_punctuation_and_modifiers() {
        let bindings = Bindings::default();
        assert_eq!(
            bindings.action(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT)),
            Some(Control::Mode)
        );
        assert_eq!(
            bindings.action(KeyEvent::new(KeyCode::Char('+'), KeyModifiers::SHIFT)),
            Some(Control::VolumeUp)
        );
        assert_eq!(
            bindings.action(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::ALT)),
            None
        );
        assert_eq!(
            parse_key("ctrl+alt+f2").unwrap(),
            (KeyCode::F(2), KeyModifiers::CONTROL | KeyModifiers::ALT)
        );
    }
}
