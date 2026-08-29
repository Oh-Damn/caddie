#![cfg(target_os = "macos")]

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

const K_ANSI_A: u32 = 0x00;
const K_ANSI_B: u32 = 0x0B;
const K_ANSI_F: u32 = 0x03;
const K_ANSI_S: u32 = 0x01;
const K_ANSI_Z: u32 = 0x06;
const K_ANSI_X: u32 = 0x07;
const K_ANSI_C: u32 = 0x08;
const K_ANSI_V: u32 = 0x09;

pub fn play_pause() -> Result<(), String> {
    click(Key::MediaPlayPause)
}

pub fn next_track() -> Result<(), String> {
    click(Key::MediaNextTrack)
}

pub fn prev_track() -> Result<(), String> {
    click(Key::MediaPrevTrack)
}

pub fn spotify_like() -> Result<(), String> {
    chord(&[Key::Alt, Key::Shift], Key::Other(K_ANSI_B))
}

pub fn shortcut(action: &str) -> Result<(), String> {
    if let Some(spec) = action.strip_prefix("shortcut.send:") {
        return send_spec(spec);
    }
    match action {
        "shortcut.undo" => chord(&[Key::Meta], Key::Other(K_ANSI_Z)),
        "shortcut.redo" => chord(&[Key::Meta, Key::Shift], Key::Other(K_ANSI_Z)),
        "shortcut.cut" => chord(&[Key::Meta], Key::Other(K_ANSI_X)),
        "shortcut.copy" => chord(&[Key::Meta], Key::Other(K_ANSI_C)),
        "shortcut.paste" => chord(&[Key::Meta], Key::Other(K_ANSI_V)),
        "shortcut.select_all" => chord(&[Key::Meta], Key::Other(K_ANSI_A)),
        "shortcut.save" => chord(&[Key::Meta], Key::Other(K_ANSI_S)),
        "shortcut.find" => chord(&[Key::Meta], Key::Other(K_ANSI_F)),
        _ => Err(format!("unknown shortcut {action}")),
    }
}

fn send_spec(spec: &str) -> Result<(), String> {
    let (mods, code) = crate::chords::parse_chord(spec)?;
    let modifiers: Vec<Key> = mods
        .into_iter()
        .map(|m| match m {
            crate::chords::Modifier::Meta => Key::Meta,
            crate::chords::Modifier::Shift => Key::Shift,
            crate::chords::Modifier::Alt => Key::Alt,
            crate::chords::Modifier::Control => Key::Control,
        })
        .collect();
    chord(&modifiers, Key::Other(code))
}

pub fn type_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    if !crate::os::permissions::accessibility_trusted() {
        return Err(format!(
            "Enable Accessibility for {} in System Settings > Privacy & Security > Accessibility",
            crate::protocol::PRODUCT_NAME
        ));
    }
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.text(text).map_err(|e| e.to_string())?;
    Ok(())
}

fn click(key: Key) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo
        .key(key, Direction::Click)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn chord(modifiers: &[Key], key: Key) -> Result<(), String> {
    if !crate::os::permissions::accessibility_trusted() {
        return Err(format!(
            "Enable Accessibility for {} in System Settings > Privacy & Security > Accessibility",
            crate::protocol::PRODUCT_NAME
        ));
    }
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    for m in modifiers {
        enigo.key(*m, Direction::Press).map_err(|e| e.to_string())?;
    }
    let clicked = enigo.key(key, Direction::Click);
    for m in modifiers.iter().rev() {
        let _ = enigo.key(*m, Direction::Release);
    }
    clicked.map_err(|e| e.to_string())
}
