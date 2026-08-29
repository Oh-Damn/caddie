#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Meta,
    Shift,
    Alt,
    Control,
}

pub fn parse_chord(spec: &str) -> Result<(Vec<Modifier>, u32), String> {
    let parts: Vec<&str> = spec
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return Err("empty chord".into());
    }
    let (key_tok, mod_toks) = parts.split_last().unwrap();
    let mut modifiers = Vec::new();
    for tok in mod_toks {
        modifiers.push(parse_modifier(tok)?);
    }
    let code = keycode(key_tok)?;
    Ok((modifiers, code))
}

fn parse_modifier(tok: &str) -> Result<Modifier, String> {
    match tok.to_ascii_lowercase().as_str() {
        "meta" | "cmd" | "command" | "super" => Ok(Modifier::Meta),
        "shift" => Ok(Modifier::Shift),
        "alt" | "option" => Ok(Modifier::Alt),
        "ctrl" | "control" => Ok(Modifier::Control),
        other => Err(format!("unknown modifier {other}")),
    }
}

fn keycode(tok: &str) -> Result<u32, String> {
    let t = tok.to_ascii_lowercase();
    let code = match t.as_str() {
        "a" => 0x00,
        "s" => 0x01,
        "d" => 0x02,
        "f" => 0x03,
        "h" => 0x04,
        "g" => 0x05,
        "z" => 0x06,
        "x" => 0x07,
        "c" => 0x08,
        "v" => 0x09,
        "b" => 0x0b,
        "q" => 0x0c,
        "w" => 0x0d,
        "e" => 0x0e,
        "r" => 0x0f,
        "y" => 0x10,
        "t" => 0x11,
        "1" => 0x12,
        "2" => 0x13,
        "3" => 0x14,
        "4" => 0x15,
        "6" => 0x16,
        "5" => 0x17,
        "9" => 0x19,
        "7" => 0x1a,
        "8" => 0x1c,
        "0" => 0x1d,
        "o" => 0x1f,
        "u" => 0x20,
        "i" => 0x22,
        "p" => 0x23,
        "l" => 0x25,
        "j" => 0x26,
        "k" => 0x28,
        "n" => 0x2d,
        "m" => 0x2e,
        "slash" | "/" => 0x2c,
        "comma" | "," => 0x2b,
        "period" | "." => 0x2f,
        "grave" | "`" | "backtick" => 0x32,
        "leftbracket" | "[" => 0x21,
        "rightbracket" | "]" => 0x1e,
        "space" => 0x31,
        "tab" => 0x30,
        "enter" | "return" => 0x24,
        "escape" | "esc" => 0x35,
        "backspace" | "delete" => 0x33,
        "f1" => 0x7a,
        "f2" => 0x78,
        "f3" => 0x63,
        "f4" => 0x76,
        "f5" => 0x60,
        "f6" => 0x61,
        "f7" => 0x62,
        "f8" => 0x64,
        "f9" => 0x65,
        "f10" => 0x6d,
        "f11" => 0x67,
        "f12" => 0x6f,
        _ => return Err(format!("unknown key {tok}")),
    };
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::{parse_chord, Modifier};

    #[test]
    fn parses_meta_shift_letter() {
        let (mods, code) = parse_chord("meta+shift+m").unwrap();
        assert_eq!(mods, vec![Modifier::Meta, Modifier::Shift]);
        assert_eq!(code, 0x2e);
    }

    #[test]
    fn parses_control_grave() {
        let (mods, code) = parse_chord("control+grave").unwrap();
        assert_eq!(mods, vec![Modifier::Control]);
        assert_eq!(code, 0x32);
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_chord("").is_err());
    }

    #[test]
    fn parses_meta_backspace() {
        let (mods, code) = parse_chord("meta+shift+backspace").unwrap();
        assert_eq!(mods, vec![Modifier::Meta, Modifier::Shift]);
        assert_eq!(code, 0x33);
    }
}
