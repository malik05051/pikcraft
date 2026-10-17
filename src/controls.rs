//! Keyboard layouts (QWERTY / AZERTY) and persisted player settings.
//!
//! miniquad reports keys differently per platform: on Windows a `KeyCode` is the *physical*
//! key position (scancode, named after US QWERTY), while on Linux/X11 it is the *symbol* the
//! active layout prints on the key. Bindings here are written as the letters the player sees,
//! and `code()` converts them to whatever the platform reports.

use macroquad::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Layout {
    Qwerty,
    Azerty,
}

impl Layout {
    pub fn name(self) -> &'static str {
        match self {
            Layout::Qwerty => "QWERTY",
            Layout::Azerty => "AZERTY",
        }
    }

    pub fn move_keys(self) -> &'static str {
        match self {
            Layout::Qwerty => "WASD",
            Layout::Azerty => "ZQSD",
        }
    }

    pub fn toggled(self) -> Layout {
        match self {
            Layout::Qwerty => Layout::Azerty,
            Layout::Azerty => Layout::Qwerty,
        }
    }

    pub fn forward(self) -> KeyCode {
        if self == Layout::Azerty {
            KeyCode::Z
        } else {
            KeyCode::W
        }
    }

    pub fn left(self) -> KeyCode {
        if self == Layout::Azerty {
            KeyCode::Q
        } else {
            KeyCode::A
        }
    }

    /// The `KeyCode` the platform reports for the key labelled `letter` on this layout.
    pub fn code(self, letter: KeyCode) -> KeyCode {
        if !cfg!(windows) || self == Layout::Qwerty {
            return letter;
        }
        // Windows reports US-QWERTY positions: map AZERTY labels to where they sit on QWERTY.
        match letter {
            KeyCode::A => KeyCode::Q,
            KeyCode::Q => KeyCode::A,
            KeyCode::Z => KeyCode::W,
            KeyCode::W => KeyCode::Z,
            KeyCode::M => KeyCode::Semicolon,
            other => other,
        }
    }

    pub fn down(self, letter: KeyCode) -> bool {
        is_key_down(self.code(letter))
    }

    pub fn pressed(self, letter: KeyCode) -> bool {
        is_key_pressed(self.code(letter))
    }

    /// Best guess from the operating system's keyboard / language settings.
    pub fn detect() -> Layout {
        #[cfg(windows)]
        {
            #[link(name = "user32")]
            unsafe extern "system" {
                fn GetKeyboardLayout(thread: u32) -> isize;
            }
            // The high word of the HKL is the layout id; French (France/Belgium/Monaco) are AZERTY.
            let hkl = unsafe { GetKeyboardLayout(0) } as usize;
            let device = (hkl >> 16) & 0xFFFF;
            let lang = hkl & 0xFFFF;
            if [0x040C, 0x080C, 0x180C].contains(&device)
                || [0x040C, 0x080C, 0x180C].contains(&lang)
            {
                return Layout::Azerty;
            }
            Layout::Qwerty
        }
        #[cfg(not(windows))]
        {
            for var in ["XKB_DEFAULT_LAYOUT", "LC_ALL", "LC_CTYPE", "LANG"] {
                if let Ok(v) = std::env::var(var) {
                    let v = v.to_ascii_lowercase();
                    if v.is_empty() {
                        continue;
                    }
                    let azerty =
                        v == "fr" || v == "be" || v.starts_with("fr_fr") || v.starts_with("fr_be");
                    return if azerty {
                        Layout::Azerty
                    } else {
                        Layout::Qwerty
                    };
                }
            }
            Layout::Qwerty
        }
    }
}

/// Characters the number row produces without Shift on an AZERTY keyboard (1..9).
const AZERTY_DIGITS: [char; 9] = ['&', 'é', '"', '\'', '(', '-', 'è', '_', 'ç'];

const DIGIT_KEYS: [KeyCode; 9] = [
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
];

/// Number row press this frame as an index 0..9, whatever the layout. `chars` must be the
/// characters typed this frame (macroquad's char queue can only be drained once).
pub fn digit_pressed(chars: &[char]) -> Option<usize> {
    if let Some(i) = DIGIT_KEYS.iter().position(|k| is_key_pressed(*k)) {
        return Some(i);
    }
    chars.iter().find_map(|c| {
        if let Some(d) = c.to_digit(10) {
            return (1..=9).contains(&d).then(|| d as usize - 1);
        }
        AZERTY_DIGITS.iter().position(|a| a == c)
    })
}

/// Drain the characters typed since the last frame.
pub fn typed_chars() -> Vec<char> {
    let mut v = Vec::new();
    while let Some(c) = get_char_pressed() {
        v.push(c);
    }
    v
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub layout: Layout,
    pub music: bool,
    pub sounds: bool,
    /// Check GitHub for a newer version at startup.
    pub updates: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            layout: Layout::detect(),
            music: true,
            sounds: true,
            updates: true,
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map(|b| b.join("PikCraft").join("settings.txt"))
}

impl Settings {
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "layout" if v.eq_ignore_ascii_case("azerty") => s.layout = Layout::Azerty,
                "layout" if v.eq_ignore_ascii_case("qwerty") => s.layout = Layout::Qwerty,
                "music" => s.music = v != "off",
                "sounds" => s.sounds = v != "off",
                "updates" => s.updates = v != "off",
                _ => {}
            }
        }
        s
    }

    pub fn to_text(&self) -> String {
        let onoff = |b: bool| if b { "on" } else { "off" };
        format!(
            "# PikCraft settings\nlayout = {}\nmusic = {}\nsounds = {}\nupdates = {}\n",
            self.layout.name().to_lowercase(),
            onoff(self.music),
            onoff(self.sounds),
            onoff(self.updates)
        )
    }

    pub fn load() -> Settings {
        settings_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Settings::parse(&t))
            .unwrap_or_default()
    }

    /// Best effort: settings simply aren't remembered if the config folder isn't writable.
    pub fn save(&self) {
        if let Some(p) = settings_path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, self.to_text());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let s = Settings {
            layout: Layout::Azerty,
            music: false,
            sounds: true,
            updates: false,
        };
        assert_eq!(Settings::parse(&s.to_text()), s);
        let q = Settings {
            layout: Layout::Qwerty,
            music: true,
            sounds: false,
            updates: true,
        };
        assert_eq!(Settings::parse(&q.to_text()), q);
    }

    #[test]
    fn azerty_bindings() {
        assert_eq!(Layout::Azerty.forward(), KeyCode::Z);
        assert_eq!(Layout::Azerty.left(), KeyCode::Q);
        assert_eq!(Layout::Qwerty.forward(), KeyCode::W);
        // Letters shared by both layouts never move.
        for k in [
            KeyCode::E,
            KeyCode::R,
            KeyCode::F,
            KeyCode::X,
            KeyCode::C,
            KeyCode::L,
        ] {
            assert_eq!(Layout::Azerty.code(k), k);
        }
        if cfg!(windows) {
            assert_eq!(Layout::Azerty.code(KeyCode::Z), KeyCode::W);
        } else {
            assert_eq!(Layout::Azerty.code(KeyCode::Z), KeyCode::Z);
        }
    }

    #[test]
    fn azerty_number_row_chars() {
        // digit_pressed's char path (no keys are down in tests).
        assert_eq!(AZERTY_DIGITS.iter().position(|c| *c == 'é'), Some(1));
        assert_eq!(AZERTY_DIGITS.len(), 9);
    }
}
