use anyhow::{anyhow, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone)]
pub struct KeyBindings(pub Vec<KeyBinding>);

fn parse_keycode(s: &str) -> Result<KeyCode> {
    match s {
        "esc" => Ok(KeyCode::Esc),
        "enter" => Ok(KeyCode::Enter),
        "space" => Ok(KeyCode::Char(' ')),
        "backspace" => Ok(KeyCode::Backspace),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "pageup" => Ok(KeyCode::PageUp),
        "pagedown" => Ok(KeyCode::PageDown),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "tab" => Ok(KeyCode::Tab),
        s if s.chars().count() == 1 => Ok(KeyCode::Char(s.chars().next().unwrap())),
        other => Err(anyhow!("unknown key: {:?}", other)),
    }
}

impl KeyBinding {
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(anyhow!("key binding string is empty"));
        }

        if let Some(rest) = s.strip_prefix("ctrl+") {
            if rest.is_empty() {
                return Err(anyhow!("nothing after 'ctrl+' in key binding {:?}", s));
            }
            let code = parse_keycode(rest)?;
            return Ok(KeyBinding {
                code,
                modifiers: KeyModifiers::CONTROL,
            });
        }

        let code = parse_keycode(s)?;
        Ok(KeyBinding {
            code,
            modifiers: KeyModifiers::NONE,
        })
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        if event.code != self.code {
            return false;
        }
        if self.modifiers == KeyModifiers::NONE {
            // Allow SHIFT (terminals may report it for uppercase chars), but reject CONTROL
            !event.modifiers.contains(KeyModifiers::CONTROL)
        } else {
            event.modifiers.contains(self.modifiers)
        }
    }
}

impl KeyBindings {
    pub fn matches(&self, event: &KeyEvent) -> bool {
        self.0.iter().any(|kb| kb.matches(event))
    }

}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

pub fn deserialize_key_option<'de, D>(deserializer: D) -> Result<Option<KeyBindings>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<StringOrVec> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(StringOrVec::Single(s)) => {
            let kb = KeyBinding::parse(&s).map_err(serde::de::Error::custom)?;
            Ok(Some(KeyBindings(vec![kb])))
        }
        Some(StringOrVec::Multiple(vec)) => {
            let bindings = vec
                .into_iter()
                .map(|s| KeyBinding::parse(&s))
                .collect::<Result<Vec<_>>>()
                .map_err(serde::de::Error::custom)?;
            Ok(Some(KeyBindings(bindings)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_char() {
        let kb = KeyBinding::parse("q").unwrap();
        assert_eq!(kb.code, KeyCode::Char('q'));
        assert_eq!(kb.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn parse_uppercase_char() {
        let kb = KeyBinding::parse("G").unwrap();
        assert_eq!(kb.code, KeyCode::Char('G'));
        assert_eq!(kb.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn parse_ctrl_key() {
        let kb = KeyBinding::parse("ctrl+d").unwrap();
        assert_eq!(kb.code, KeyCode::Char('d'));
        assert_eq!(kb.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn parse_special_keys() {
        assert_eq!(KeyBinding::parse("esc").unwrap().code, KeyCode::Esc);
        assert_eq!(KeyBinding::parse("enter").unwrap().code, KeyCode::Enter);
        assert_eq!(KeyBinding::parse("space").unwrap().code, KeyCode::Char(' '));
        assert_eq!(KeyBinding::parse("backspace").unwrap().code, KeyCode::Backspace);
        assert_eq!(KeyBinding::parse("pageup").unwrap().code, KeyCode::PageUp);
        assert_eq!(KeyBinding::parse("pagedown").unwrap().code, KeyCode::PageDown);
        assert_eq!(KeyBinding::parse("home").unwrap().code, KeyCode::Home);
        assert_eq!(KeyBinding::parse("end").unwrap().code, KeyCode::End);
    }

    #[test]
    fn parse_arrow_keys() {
        assert_eq!(KeyBinding::parse("up").unwrap().code, KeyCode::Up);
        assert_eq!(KeyBinding::parse("down").unwrap().code, KeyCode::Down);
        assert_eq!(KeyBinding::parse("left").unwrap().code, KeyCode::Left);
        assert_eq!(KeyBinding::parse("right").unwrap().code, KeyCode::Right);
    }

    #[test]
    fn parse_symbols() {
        assert_eq!(KeyBinding::parse("/").unwrap().code, KeyCode::Char('/'));
        assert_eq!(KeyBinding::parse("?").unwrap().code, KeyCode::Char('?'));
        assert_eq!(KeyBinding::parse("$").unwrap().code, KeyCode::Char('$'));
        assert_eq!(KeyBinding::parse("0").unwrap().code, KeyCode::Char('0'));
    }

    #[test]
    fn parse_invalid_key() {
        assert!(KeyBinding::parse("ctrl+").is_err());
        assert!(KeyBinding::parse("").is_err());
        assert!(KeyBinding::parse("ctrl+abc").is_err());
    }

    #[test]
    fn matches_simple_char() {
        let kb = KeyBinding::parse("q").unwrap();
        let event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(kb.matches(&event));
    }

    #[test]
    fn matches_char_ignores_shift() {
        let kb = KeyBinding::parse("G").unwrap();
        let event = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert!(kb.matches(&event));
    }

    #[test]
    fn matches_ctrl_key() {
        let kb = KeyBinding::parse("ctrl+d").unwrap();
        let yes = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        let no = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        assert!(kb.matches(&yes));
        assert!(!kb.matches(&no));
    }

    #[test]
    fn simple_char_does_not_match_ctrl_variant() {
        let kb = KeyBinding::parse("d").unwrap();
        let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(!kb.matches(&ctrl_d));
    }

    #[test]
    fn keybindings_matches_any() {
        let bindings = KeyBindings(vec![
            KeyBinding::parse("q").unwrap(),
            KeyBinding::parse("esc").unwrap(),
        ]);
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert!(bindings.matches(&q));
        assert!(bindings.matches(&esc));
        assert!(!bindings.matches(&j));
    }

    #[test]
    fn deserialize_single_string() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(default, deserialize_with = "super::deserialize_key_option")]
            key: Option<KeyBindings>,
        }
        let t: Test = toml::from_str(r#"key = "q""#).unwrap();
        assert_eq!(t.key.unwrap().0.len(), 1);
    }

    #[test]
    fn deserialize_array() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(default, deserialize_with = "super::deserialize_key_option")]
            key: Option<KeyBindings>,
        }
        let t: Test = toml::from_str(r#"key = ["q", "esc"]"#).unwrap();
        assert_eq!(t.key.unwrap().0.len(), 2);
    }

    #[test]
    fn deserialize_missing_key() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(default, deserialize_with = "super::deserialize_key_option")]
            key: Option<KeyBindings>,
        }
        let t: Test = toml::from_str("").unwrap();
        assert!(t.key.is_none());
    }
}
