use shared_protocol::{KeyModifier, Shortcut};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_SPACE,
};

pub(crate) fn parse_virtual_key(key: &str) -> Result<u32, String> {
    if key.eq_ignore_ascii_case("Space") {
        return Ok(VK_SPACE as u32);
    }

    if key.len() == 1 {
        let ch = key
            .chars()
            .next()
            .ok_or_else(|| "empty key binding".to_string())?
            .to_ascii_uppercase();
        if ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
            return Ok(ch as u32);
        }
    }

    Err(format!(
        "unsupported key `{key}`; current prototype supports Space, A-Z, and 0-9"
    ))
}

pub(crate) fn collect_release_virtual_keys(shortcut: &Shortcut) -> Result<Vec<i32>, String> {
    let mut virtual_keys = Vec::new();
    for modifier in &shortcut.modifiers {
        match modifier {
            KeyModifier::Control => virtual_keys.push(VK_CONTROL as i32),
            KeyModifier::Alt => virtual_keys.push(VK_MENU as i32),
            KeyModifier::Shift => virtual_keys.push(VK_SHIFT as i32),
            KeyModifier::Meta => {
                virtual_keys.push(VK_LWIN as i32);
                virtual_keys.push(VK_RWIN as i32);
            }
        }
    }

    virtual_keys.push(parse_virtual_key(&shortcut.key)? as i32);
    virtual_keys.sort_unstable();
    virtual_keys.dedup();
    Ok(virtual_keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_virtual_key() {
        assert_eq!(
            parse_virtual_key("Space").expect("space should parse"),
            VK_SPACE as u32
        );
        assert_eq!(
            parse_virtual_key("r").expect("letter should parse"),
            b'R' as u32
        );
    }

    #[test]
    fn collects_release_virtual_keys_for_shared_shortcut() {
        let shortcut = Shortcut {
            modifiers: vec![KeyModifier::Control, KeyModifier::Shift],
            key: "R".to_string(),
        };

        let release_virtual_keys =
            collect_release_virtual_keys(&shortcut).expect("shortcut should parse");

        assert!(release_virtual_keys.contains(&(b'R' as i32)));
        assert!(release_virtual_keys.contains(&(VK_SHIFT as i32)));
        assert!(release_virtual_keys.contains(&(VK_CONTROL as i32)));
    }
}
