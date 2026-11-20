//! Parser and exporter for `import.md` macro text command sequences.
//!
//! Supports standard keyboard/mouse tags (`{CTRL}`, `{HOLDMS:100}`, `{WAITMS:50}`,
//! `{PRESS}`, `{RELEASE}`, `{LMBD}`, `{LMBU}`, etc.), hold-group export optimization,
//! and multi-track parallel interval allocation.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::domain::keys::{InputSource, KeyboardKey, MouseButton, ScanCode};
use crate::domain::macros::{
    allocate_tracks_greedy, keyboard_key_short_label, Clip, ClipKind, DEFAULT_CLIP_MS, MAX_MS,
};

/// Result of parsing an `import.md` macro text string into multi-track clips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMacroSequence {
    pub clips: Vec<Clip>,
    pub max_concurrent_tracks: usize,
    pub total_ms: u64,
    pub notes: Vec<String>,
    pub system_tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PressReleaseMode {
    Press,
    Release,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Tag(String),
    Char(char),
    LiteralDelete,
}

#[derive(Debug, Clone)]
struct RawClipEvent {
    order: usize,
    clip: Clip,
}

#[derive(Debug, Clone)]
struct ExportEvent {
    t: u64,
    order: u8,
    track: isize,
    text: String,
    advance_ms: u64,
}

/// Converts a [`KeyboardKey`] to its `import.md` tag or character representation.
pub fn keyboard_key_to_import_token(key: KeyboardKey) -> String {
    match key {
        KeyboardKey::LeftCtrl => "{CTRL}".to_string(),
        KeyboardKey::RightCtrl => "{RCTRL}".to_string(),
        KeyboardKey::LeftAlt => "{ALT}".to_string(),
        KeyboardKey::RightAlt => "{RALT}".to_string(),
        KeyboardKey::LeftShift => "{SHIFT}".to_string(),
        KeyboardKey::RightShift => "{RSHIFT}".to_string(),
        KeyboardKey::LeftMeta => "{LWIN}".to_string(),
        KeyboardKey::RightMeta => "{RWIN}".to_string(),
        KeyboardKey::ContextMenu => "{APPS}".to_string(),
        KeyboardKey::Delete => "{DELETE}".to_string(),
        KeyboardKey::Insert => "{INS}".to_string(),
        KeyboardKey::PageUp => "{PGUP}".to_string(),
        KeyboardKey::PageDown => "{PGDN}".to_string(),
        KeyboardKey::Home => "{HOME}".to_string(),
        KeyboardKey::End => "{END}".to_string(),
        KeyboardKey::Enter => "{ENTER}".to_string(),
        KeyboardKey::Escape => "{ESC}".to_string(),
        KeyboardKey::Backspace => "{BACKSPACE}".to_string(),
        KeyboardKey::Tab => "{TAB}".to_string(),
        KeyboardKey::PrintScreen => "{PRTSCN}".to_string(),
        KeyboardKey::Pause => "{PAUSE}".to_string(),
        KeyboardKey::Space => "{SPACE}".to_string(),
        KeyboardKey::CapsLock => "{CAPSLOCK}".to_string(),
        KeyboardKey::NumLock => "{NUMLOCK}".to_string(),
        KeyboardKey::ScrollLock => "{SCROLLLOCK}".to_string(),
        KeyboardKey::ArrowUp => "{UP}".to_string(),
        KeyboardKey::ArrowDown => "{DOWN}".to_string(),
        KeyboardKey::ArrowLeft => "{LEFT}".to_string(),
        KeyboardKey::ArrowRight => "{RIGHT}".to_string(),
        KeyboardKey::F1 => "{F1}".to_string(),
        KeyboardKey::F2 => "{F2}".to_string(),
        KeyboardKey::F3 => "{F3}".to_string(),
        KeyboardKey::F4 => "{F4}".to_string(),
        KeyboardKey::F5 => "{F5}".to_string(),
        KeyboardKey::F6 => "{F6}".to_string(),
        KeyboardKey::F7 => "{F7}".to_string(),
        KeyboardKey::F8 => "{F8}".to_string(),
        KeyboardKey::F9 => "{F9}".to_string(),
        KeyboardKey::F10 => "{F10}".to_string(),
        KeyboardKey::F11 => "{F11}".to_string(),
        KeyboardKey::F12 => "{F12}".to_string(),
        KeyboardKey::F13 => "{F13}".to_string(),
        KeyboardKey::F14 => "{F14}".to_string(),
        KeyboardKey::F15 => "{F15}".to_string(),
        KeyboardKey::F16 => "{F16}".to_string(),
        KeyboardKey::F17 => "{F17}".to_string(),
        KeyboardKey::F18 => "{F18}".to_string(),
        KeyboardKey::F19 => "{F19}".to_string(),
        KeyboardKey::F20 => "{F20}".to_string(),
        KeyboardKey::F21 => "{F21}".to_string(),
        KeyboardKey::F22 => "{F22}".to_string(),
        KeyboardKey::F23 => "{F23}".to_string(),
        KeyboardKey::F24 => "{F24}".to_string(),
        KeyboardKey::Numpad0 => "{NUM0}".to_string(),
        KeyboardKey::Numpad1 => "{NUM1}".to_string(),
        KeyboardKey::Numpad2 => "{NUM2}".to_string(),
        KeyboardKey::Numpad3 => "{NUM3}".to_string(),
        KeyboardKey::Numpad4 => "{NUM4}".to_string(),
        KeyboardKey::Numpad5 => "{NUM5}".to_string(),
        KeyboardKey::Numpad6 => "{NUM6}".to_string(),
        KeyboardKey::Numpad7 => "{NUM7}".to_string(),
        KeyboardKey::Numpad8 => "{NUM8}".to_string(),
        KeyboardKey::Numpad9 => "{NUM9}".to_string(),
        KeyboardKey::NumpadAdd => "{NUM+}".to_string(),
        KeyboardKey::NumpadSubtract => "{NUM-}".to_string(),
        KeyboardKey::NumpadDecimal => "{NUM.}".to_string(),
        KeyboardKey::NumpadDivide => "{NUM/}".to_string(),
        KeyboardKey::NumpadMultiply => "{NUM*}".to_string(),
        KeyboardKey::NumpadEnter => "{NUMENTER}".to_string(),
        KeyboardKey::VolumeUp => "{VOL+}".to_string(),
        KeyboardKey::VolumeDown => "{VOL-}".to_string(),
        KeyboardKey::VolumeMute => "{MUTE}".to_string(),
        KeyboardKey::MediaPlayPause => "{MEDIAPLAY}".to_string(),
        KeyboardKey::MediaStop => "{MEDIASTOP}".to_string(),
        KeyboardKey::MediaNext => "{MEDIANEXT}".to_string(),
        KeyboardKey::MediaPrev => "{MEDIAPREV}".to_string(),
        KeyboardKey::KeyA => "a".to_string(),
        KeyboardKey::KeyB => "b".to_string(),
        KeyboardKey::KeyC => "c".to_string(),
        KeyboardKey::KeyD => "d".to_string(),
        KeyboardKey::KeyE => "e".to_string(),
        KeyboardKey::KeyF => "f".to_string(),
        KeyboardKey::KeyG => "g".to_string(),
        KeyboardKey::KeyH => "h".to_string(),
        KeyboardKey::KeyI => "i".to_string(),
        KeyboardKey::KeyJ => "j".to_string(),
        KeyboardKey::KeyK => "k".to_string(),
        KeyboardKey::KeyL => "l".to_string(),
        KeyboardKey::KeyM => "m".to_string(),
        KeyboardKey::KeyN => "n".to_string(),
        KeyboardKey::KeyO => "o".to_string(),
        KeyboardKey::KeyP => "p".to_string(),
        KeyboardKey::KeyQ => "q".to_string(),
        KeyboardKey::KeyR => "r".to_string(),
        KeyboardKey::KeyS => "s".to_string(),
        KeyboardKey::KeyT => "t".to_string(),
        KeyboardKey::KeyU => "u".to_string(),
        KeyboardKey::KeyV => "v".to_string(),
        KeyboardKey::KeyW => "w".to_string(),
        KeyboardKey::KeyX => "x".to_string(),
        KeyboardKey::KeyY => "y".to_string(),
        KeyboardKey::KeyZ => "z".to_string(),
        KeyboardKey::Digit0 => "0".to_string(),
        KeyboardKey::Digit1 => "1".to_string(),
        KeyboardKey::Digit2 => "2".to_string(),
        KeyboardKey::Digit3 => "3".to_string(),
        KeyboardKey::Digit4 => "4".to_string(),
        KeyboardKey::Digit5 => "5".to_string(),
        KeyboardKey::Digit6 => "6".to_string(),
        KeyboardKey::Digit7 => "7".to_string(),
        KeyboardKey::Digit8 => "8".to_string(),
        KeyboardKey::Digit9 => "9".to_string(),
        KeyboardKey::Grave => "`".to_string(),
        KeyboardKey::Minus => "-".to_string(),
        KeyboardKey::Equal => "=".to_string(),
        KeyboardKey::LeftBracket => "[".to_string(),
        KeyboardKey::RightBracket => "]".to_string(),
        KeyboardKey::Backslash => "\\".to_string(),
        KeyboardKey::Semicolon => ";".to_string(),
        KeyboardKey::Quote => "'".to_string(),
        KeyboardKey::Comma => ",".to_string(),
        KeyboardKey::Period => ".".to_string(),
        KeyboardKey::Slash => "/".to_string(),
        KeyboardKey::OemScanCode(sc) => {
            if sc.extended {
                format!("{{SCE:0x{:X}}}", sc.code)
            } else {
                format!("{{SC:0x{:X}}}", sc.code)
            }
        }
    }
}

const fn is_modifier_key(key: KeyboardKey) -> bool {
    matches!(
        key,
        KeyboardKey::LeftCtrl
            | KeyboardKey::RightCtrl
            | KeyboardKey::LeftAlt
            | KeyboardKey::RightAlt
            | KeyboardKey::LeftShift
            | KeyboardKey::RightShift
            | KeyboardKey::LeftMeta
            | KeyboardKey::RightMeta
    )
}

const fn mouse_down_tag(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "{LMBD}",
        MouseButton::Right => "{RMBD}",
        MouseButton::Middle => "{MMBD}",
        MouseButton::XButton1 => "{MB4D}",
        MouseButton::XButton2 => "{MB5D}",
    }
}

const fn mouse_up_tag(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "{LMBU}",
        MouseButton::Right => "{RMBU}",
        MouseButton::Middle => "{MMBU}",
        MouseButton::XButton1 => "{MB4U}",
        MouseButton::XButton2 => "{MB5U}",
    }
}

/// Exports timeline clips to an `import.md` macro text command sequence.
///
/// Applies `{HOLDMS:dur}` hold-group optimization when a synchronized group of clips
/// shares identical `[start, end)` bounds, consists of one primary key plus optional
/// distinct modifiers, and has no overlapping clip boundaries inside `(start, end)`.
pub fn export_clips_to_macro_text(clips: &[Clip]) -> String {
    let valid_clips: Vec<&Clip> = clips.iter().filter(|c| !c.is_unassigned_key()).collect();
    if valid_clips.is_empty() {
        return String::new();
    }

    let mut groups: BTreeMap<(u64, u64), Vec<&Clip>> = BTreeMap::new();
    for &clip in &valid_clips {
        groups
            .entry((clip.start_ms, clip.end_ms()))
            .or_default()
            .push(clip);
    }

    let mut consumed_ids: HashSet<u32> = HashSet::new();
    let mut events: Vec<ExportEvent> = Vec::new();

    for (&(s, e), group) in &groups {
        let all_keys = group
            .iter()
            .all(|c| matches!(c.kind, ClipKind::Key { key: Some(_) }));
        if !all_keys {
            continue;
        }

        let mut mods = Vec::new();
        let mut mains = Vec::new();
        let mut seen_mods = HashSet::new();
        let mut duplicate_mod = false;

        for &clip in group {
            if let ClipKind::Key { key: Some(k) } = clip.kind {
                if is_modifier_key(k) {
                    if !seen_mods.insert(k) {
                        duplicate_mod = true;
                        break;
                    }
                    mods.push(k);
                } else {
                    mains.push(k);
                }
            }
        }

        if duplicate_mod || mains.len() != 1 {
            continue;
        }

        let group_ids: HashSet<u32> = group.iter().map(|c| c.id).collect();
        let blocked_by_other = valid_clips.iter().any(|other| {
            if group_ids.contains(&other.id) {
                return false;
            }
            let os = other.start_ms;
            let oe = other.end_ms();
            os == s || oe == e || (os > s && os < e) || (oe > s && oe < e)
        });
        if blocked_by_other {
            continue;
        }

        let dur = e.saturating_sub(s).max(1);
        let mods_str: String = mods
            .iter()
            .map(|&m| keyboard_key_to_import_token(m))
            .collect();
        let main_str = keyboard_key_to_import_token(mains[0]);
        events.push(ExportEvent {
            t: s,
            order: 1,
            track: 0,
            text: format!("{{HOLDMS:{dur}}}{mods_str}{main_str}"),
            advance_ms: dur,
        });
        consumed_ids.extend(group_ids);
    }

    for &clip in &valid_clips {
        if consumed_ids.contains(&clip.id) {
            continue;
        }
        match clip.kind {
            ClipKind::Key { key: Some(key) } => {
                let tok = keyboard_key_to_import_token(key);
                events.push(ExportEvent {
                    t: clip.start_ms,
                    order: 2,
                    track: clip.track as isize,
                    text: format!("{{PRESS}}{tok}"),
                    advance_ms: 0,
                });
                events.push(ExportEvent {
                    t: clip.end_ms(),
                    order: 0,
                    track: -(clip.track as isize),
                    text: format!("{{RELEASE}}{tok}"),
                    advance_ms: 0,
                });
            }
            ClipKind::Key { key: None } => {}
            ClipKind::Mouse { button } => {
                events.push(ExportEvent {
                    t: clip.start_ms,
                    order: 2,
                    track: clip.track as isize,
                    text: mouse_down_tag(button).to_string(),
                    advance_ms: 0,
                });
                events.push(ExportEvent {
                    t: clip.end_ms(),
                    order: 0,
                    track: -(clip.track as isize),
                    text: mouse_up_tag(button).to_string(),
                    advance_ms: 0,
                });
            }
        }
    }

    events.sort_by(|a, b| {
        a.t.cmp(&b.t)
            .then_with(|| a.order.cmp(&b.order))
            .then_with(|| a.track.cmp(&b.track))
    });

    let mut out = String::new();
    let mut cursor = 0_u64;
    for ev in events {
        let gap = ev.t.saturating_sub(cursor);
        if gap > 0 {
            out.push_str(&format!("{{WAITMS:{gap}}}"));
        }
        out.push_str(&ev.text);
        cursor = cursor.max(ev.t).saturating_add(ev.advance_ms);
    }

    out
}

fn tokenize_macro_text(raw: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = raw.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '\r' || ch == '\n' {
            i += 1;
            continue;
        }
        if ch == '}' {
            return Err("Lỗi cú pháp: dấu '}' thừa không có '{' mở tương ứng.".to_string());
        }
        if ch == '{' {
            let mut end = i + 1;
            while end < chars.len() && chars[end] != '}' {
                if chars[end] == '{' {
                    return Err("Lỗi cú pháp: phát hiện '{' lồng nhau chưa đóng.".to_string());
                }
                end += 1;
            }
            if end >= chars.len() {
                return Err("Lỗi cú pháp: thẻ '{...' chưa được đóng bằng '}'.".to_string());
            }
            let inner: String = chars[i + 1..end].iter().collect();
            let trimmed = inner.trim();
            if trimmed.is_empty() {
                return Err("Lỗi cú pháp: thẻ '{}' rỗng.".to_string());
            }
            tokens.push(Token::Tag(trimmed.to_string()));
            i = end + 1;
            continue;
        }

        // Special case per import.md: literal `DELETE` right after {CTRL}/{ALT}/{PRESS}/{RELEASE}
        if matches!(tokens.last(), Some(Token::Tag(tag)) if matches!(
            tag.to_ascii_uppercase().as_str(),
            "CTRL" | "RCTRL" | "ALT" | "RALT" | "SHIFT" | "RSHIFT" | "PRESS" | "RELEASE"
        )) && i + 6 <= chars.len()
        {
            let slice: String = chars[i..i + 6].iter().collect();
            if slice.eq_ignore_ascii_case("DELETE") {
                tokens.push(Token::LiteralDelete);
                i += 6;
                continue;
            }
        }

        tokens.push(Token::Char(ch));
        i += 1;
    }

    Ok(tokens)
}

fn parse_number_u64(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else {
        trimmed.parse::<u64>().ok()
    }
}

fn parse_char_to_key(ch: char) -> Option<KeyboardKey> {
    match ch.to_ascii_lowercase() {
        'a' => Some(KeyboardKey::KeyA),
        'b' => Some(KeyboardKey::KeyB),
        'c' => Some(KeyboardKey::KeyC),
        'd' => Some(KeyboardKey::KeyD),
        'e' => Some(KeyboardKey::KeyE),
        'f' => Some(KeyboardKey::KeyF),
        'g' => Some(KeyboardKey::KeyG),
        'h' => Some(KeyboardKey::KeyH),
        'i' => Some(KeyboardKey::KeyI),
        'j' => Some(KeyboardKey::KeyJ),
        'k' => Some(KeyboardKey::KeyK),
        'l' => Some(KeyboardKey::KeyL),
        'm' => Some(KeyboardKey::KeyM),
        'n' => Some(KeyboardKey::KeyN),
        'o' => Some(KeyboardKey::KeyO),
        'p' => Some(KeyboardKey::KeyP),
        'q' => Some(KeyboardKey::KeyQ),
        'r' => Some(KeyboardKey::KeyR),
        's' => Some(KeyboardKey::KeyS),
        't' => Some(KeyboardKey::KeyT),
        'u' => Some(KeyboardKey::KeyU),
        'v' => Some(KeyboardKey::KeyV),
        'w' => Some(KeyboardKey::KeyW),
        'x' => Some(KeyboardKey::KeyX),
        'y' => Some(KeyboardKey::KeyY),
        'z' => Some(KeyboardKey::KeyZ),
        '0' => Some(KeyboardKey::Digit0),
        '1' => Some(KeyboardKey::Digit1),
        '2' => Some(KeyboardKey::Digit2),
        '3' => Some(KeyboardKey::Digit3),
        '4' => Some(KeyboardKey::Digit4),
        '5' => Some(KeyboardKey::Digit5),
        '6' => Some(KeyboardKey::Digit6),
        '7' => Some(KeyboardKey::Digit7),
        '8' => Some(KeyboardKey::Digit8),
        '9' => Some(KeyboardKey::Digit9),
        '`' => Some(KeyboardKey::Grave),
        '-' => Some(KeyboardKey::Minus),
        '=' => Some(KeyboardKey::Equal),
        '[' => Some(KeyboardKey::LeftBracket),
        ']' => Some(KeyboardKey::RightBracket),
        '\\' => Some(KeyboardKey::Backslash),
        ';' => Some(KeyboardKey::Semicolon),
        '\'' => Some(KeyboardKey::Quote),
        ',' => Some(KeyboardKey::Comma),
        '.' => Some(KeyboardKey::Period),
        '/' => Some(KeyboardKey::Slash),
        ' ' => Some(KeyboardKey::Space),
        _ => None,
    }
}

fn parse_named_key_tag(upper: &str) -> Option<KeyboardKey> {
    match upper {
        "DEL" | "DELETE" => Some(KeyboardKey::Delete),
        "INS" | "INSERT" => Some(KeyboardKey::Insert),
        "PGUP" | "PAGEUP" | "PAGE_UP" => Some(KeyboardKey::PageUp),
        "PGDN" | "PAGEDOWN" | "PAGE_DOWN" => Some(KeyboardKey::PageDown),
        "HOME" => Some(KeyboardKey::Home),
        "END" => Some(KeyboardKey::End),
        "RETURN" | "ENTER" => Some(KeyboardKey::Enter),
        "ESCAPE" | "ESC" => Some(KeyboardKey::Escape),
        "BACKSPACE" | "BS" => Some(KeyboardKey::Backspace),
        "TAB" => Some(KeyboardKey::Tab),
        "PRTSCN" | "PRINTSCREEN" | "PRINT_SCREEN" => Some(KeyboardKey::PrintScreen),
        "PAUSE" => Some(KeyboardKey::Pause),
        "SPACE" => Some(KeyboardKey::Space),
        "CAPSLOCK" | "CAPS_LOCK" => Some(KeyboardKey::CapsLock),
        "NUMLOCK" | "NUM_LOCK" => Some(KeyboardKey::NumLock),
        "SCROLLLOCK" | "SCROLL_LOCK" => Some(KeyboardKey::ScrollLock),
        "UP" => Some(KeyboardKey::ArrowUp),
        "DOWN" => Some(KeyboardKey::ArrowDown),
        "LEFT" => Some(KeyboardKey::ArrowLeft),
        "RIGHT" => Some(KeyboardKey::ArrowRight),
        "APPS" | "MENU" => Some(KeyboardKey::ContextMenu),
        "F1" => Some(KeyboardKey::F1),
        "F2" => Some(KeyboardKey::F2),
        "F3" => Some(KeyboardKey::F3),
        "F4" => Some(KeyboardKey::F4),
        "F5" => Some(KeyboardKey::F5),
        "F6" => Some(KeyboardKey::F6),
        "F7" => Some(KeyboardKey::F7),
        "F8" => Some(KeyboardKey::F8),
        "F9" => Some(KeyboardKey::F9),
        "F10" => Some(KeyboardKey::F10),
        "F11" => Some(KeyboardKey::F11),
        "F12" => Some(KeyboardKey::F12),
        "F13" => Some(KeyboardKey::F13),
        "F14" => Some(KeyboardKey::F14),
        "F15" => Some(KeyboardKey::F15),
        "F16" => Some(KeyboardKey::F16),
        "F17" => Some(KeyboardKey::F17),
        "F18" => Some(KeyboardKey::F18),
        "F19" => Some(KeyboardKey::F19),
        "F20" => Some(KeyboardKey::F20),
        "F21" => Some(KeyboardKey::F21),
        "F22" => Some(KeyboardKey::F22),
        "F23" => Some(KeyboardKey::F23),
        "F24" => Some(KeyboardKey::F24),
        "NUM0" => Some(KeyboardKey::Numpad0),
        "NUM1" => Some(KeyboardKey::Numpad1),
        "NUM2" => Some(KeyboardKey::Numpad2),
        "NUM3" => Some(KeyboardKey::Numpad3),
        "NUM4" => Some(KeyboardKey::Numpad4),
        "NUM5" => Some(KeyboardKey::Numpad5),
        "NUM6" => Some(KeyboardKey::Numpad6),
        "NUM7" => Some(KeyboardKey::Numpad7),
        "NUM8" => Some(KeyboardKey::Numpad8),
        "NUM9" => Some(KeyboardKey::Numpad9),
        "NUM+" | "NUMADD" => Some(KeyboardKey::NumpadAdd),
        "NUM-" | "NUMSUB" => Some(KeyboardKey::NumpadSubtract),
        "NUM." | "NUMDEC" => Some(KeyboardKey::NumpadDecimal),
        "NUM/" | "NUMDIV" => Some(KeyboardKey::NumpadDivide),
        "NUM*" | "NUMMUL" => Some(KeyboardKey::NumpadMultiply),
        "NUMENTER" => Some(KeyboardKey::NumpadEnter),
        "VOL+" => Some(KeyboardKey::VolumeUp),
        "VOL-" => Some(KeyboardKey::VolumeDown),
        "MUTE" => Some(KeyboardKey::VolumeMute),
        "MEDIAPLAY" => Some(KeyboardKey::MediaPlayPause),
        "MEDIASTOP" => Some(KeyboardKey::MediaStop),
        "MEDIANEXT" => Some(KeyboardKey::MediaNext),
        "MEDIAPREV" => Some(KeyboardKey::MediaPrev),
        _ => None,
    }
}

fn parse_modifier_tag(upper: &str) -> Option<KeyboardKey> {
    match upper {
        "CTRL" | "LCTRL" => Some(KeyboardKey::LeftCtrl),
        "RCTRL" => Some(KeyboardKey::RightCtrl),
        "ALT" | "LALT" => Some(KeyboardKey::LeftAlt),
        "RALT" => Some(KeyboardKey::RightAlt),
        "SHIFT" | "LSHIFT" => Some(KeyboardKey::LeftShift),
        "RSHIFT" => Some(KeyboardKey::RightShift),
        "LWIN" | "WIN" | "LMETA" => Some(KeyboardKey::LeftMeta),
        "RWIN" | "RMETA" => Some(KeyboardKey::RightMeta),
        _ => None,
    }
}

fn is_system_tag(upper: &str) -> bool {
    const EXACT: &[&str] = &[
        "MWUP",
        "MWDN",
        "TILTL",
        "TILTR",
        "OD",
        "OU",
        "OR",
        "BREAK",
        "CTRLBREAK",
        "BACK",
        "FORWARD",
        "STOP",
        "REFRESH",
        "WEBHOME",
        "SEARCH",
        "FAVORITES",
        "NUMLOCKON",
        "NUMLOCKOFF",
        "CAPSLOCKON",
        "CAPSLOCKOFF",
        "SCROLLLOCKON",
        "SCROLLLOCKOFF",
    ];
    if EXACT.contains(&upper) {
        return true;
    }
    const PREFIXES: &[&str] = &[
        "ACTIVATE", "CURSOR", "INVERT", "LOCK", "UNLOCK", "CB:", "MADD:", "MSET:", "PSET:",
        "ASET:", "MSAVE:", "MREST:", "RUN", "KILL:", "POSTWM:", "SENDWM:", "LAYER:",
    ];
    PREFIXES.iter().any(|p| upper.starts_with(p))
}

fn parse_wait_ms_value(
    arg: &str,
    is_seconds: bool,
    notes: &mut Vec<String>,
) -> Result<u64, String> {
    let trimmed = arg.trim();
    if let Some((a_raw, b_raw)) = trimmed.split_once('-') {
        let a = a_raw
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("Giá trị thời gian chờ không hợp lệ: '{trimmed}'"))?;
        let b = b_raw
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("Giá trị thời gian chờ không hợp lệ: '{trimmed}'"))?;
        let avg = ((a + b) * 0.5) * if is_seconds { 1000.0 } else { 1.0 };
        let ms = avg.round().max(0.0) as u64;
        notes.push(format!(
            "Khoảng chờ ngẫu nhiên '{trimmed}' được quy đổi về trung bình {ms} ms."
        ));
        return Ok(ms.min(MAX_MS));
    }
    let val = trimmed
        .parse::<f64>()
        .map_err(|_| format!("Giá trị thời gian không hợp lệ: '{trimmed}'"))?;
    if val < 0.0 {
        return Err(format!("Thời gian không được âm: '{trimmed}'"));
    }
    let ms = (val * if is_seconds { 1000.0 } else { 1.0 }).round() as u64;
    Ok(ms.min(MAX_MS))
}

/// Parses an `import.md` macro text command string into multi-track clips and validation info.
pub fn parse_macro_sequence(
    raw: &str,
    default_tap_ms: u64,
    next_id: &mut u32,
) -> Result<ParsedMacroSequence, String> {
    let tokens = tokenize_macro_text(raw)?;
    let tap_ms = default_tap_ms.clamp(1, MAX_MS);

    let mut t = 0_u64;
    let mut order_counter = 0_usize;
    let mut pending_modifiers: Vec<KeyboardKey> = Vec::new();
    let mut pending_hold: Option<u64> = None;
    let mut press_release_mode: Option<PressReleaseMode> = None;
    let mut open_holds: HashMap<InputSource, (usize, u64)> = HashMap::new();
    let mut raw_events: Vec<RawClipEvent> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut system_tags: Vec<String> = Vec::new();

    let mut emit_target = |target: InputSource,
                           t_now: u64,
                           pending_mods: &mut Vec<KeyboardKey>,
                           pending_h: &mut Option<u64>,
                           pr_mode: &mut Option<PressReleaseMode>,
                           open: &mut HashMap<InputSource, (usize, u64)>,
                           events: &mut Vec<RawClipEvent>|
     -> Result<u64, String> {
        if let Some(mode) = pr_mode.take() {
            match mode {
                PressReleaseMode::Press => {
                    if open.contains_key(&target) {
                        return Err(format!(
                            "'{}' đã đang được nhấn giữ ({{PRESS}}), không thể nhấn lại trước khi {{RELEASE}}.",
                            target.display_name()
                        ));
                    }
                    let ord = order_counter;
                    order_counter += 1;
                    open.insert(target, (ord, t_now));
                    return Ok(t_now);
                }
                PressReleaseMode::Release => {
                    let Some((ord, start_ms)) = open.remove(&target) else {
                        return Err(format!(
                            "'{}' chưa được nhấn giữ ({{PRESS}}) trước đó nên không thể {{RELEASE}}.",
                            target.display_name()
                        ));
                    };
                    let id = *next_id;
                    *next_id += 1;
                    let kind = match target {
                        InputSource::Keyboard(k) => ClipKind::Key { key: Some(k) },
                        InputSource::MouseButton(b) => ClipKind::Mouse { button: b },
                    };
                    events.push(RawClipEvent {
                        order: ord,
                        clip: Clip {
                            id,
                            kind,
                            track: 0,
                            start_ms,
                            dur_ms: t_now.saturating_sub(start_ms).max(1),
                        },
                    });
                    return Ok(t_now);
                }
            }
        }

        let dur = pending_h.take().unwrap_or(tap_ms).max(1);
        for mod_key in pending_mods.drain(..) {
            let id = *next_id;
            *next_id += 1;
            let ord = order_counter;
            order_counter += 1;
            events.push(RawClipEvent {
                order: ord,
                clip: Clip {
                    id,
                    kind: ClipKind::Key { key: Some(mod_key) },
                    track: 0,
                    start_ms: t_now,
                    dur_ms: dur,
                },
            });
        }
        let id = *next_id;
        *next_id += 1;
        let ord = order_counter;
        order_counter += 1;
        let kind = match target {
            InputSource::Keyboard(k) => ClipKind::Key { key: Some(k) },
            InputSource::MouseButton(b) => ClipKind::Mouse { button: b },
        };
        events.push(RawClipEvent {
            order: ord,
            clip: Clip {
                id,
                kind,
                track: 0,
                start_ms: t_now,
                dur_ms: dur,
            },
        });
        Ok(t_now.saturating_add(dur).min(MAX_MS))
    };

    for tok in tokens {
        match tok {
            Token::Char(ch) => {
                let Some(key) = parse_char_to_key(ch) else {
                    return Err(format!(
                        "Ký tự '{ch}' không được ánh xạ thành phím bàn phím."
                    ));
                };
                t = emit_target(
                    InputSource::Keyboard(key),
                    t,
                    &mut pending_modifiers,
                    &mut pending_hold,
                    &mut press_release_mode,
                    &mut open_holds,
                    &mut raw_events,
                )?;
            }
            Token::LiteralDelete => {
                t = emit_target(
                    InputSource::Keyboard(KeyboardKey::Delete),
                    t,
                    &mut pending_modifiers,
                    &mut pending_hold,
                    &mut press_release_mode,
                    &mut open_holds,
                    &mut raw_events,
                )?;
            }
            Token::Tag(tag_raw) => {
                let upper = tag_raw.to_ascii_uppercase();

                if upper == "PRESS" {
                    press_release_mode = Some(PressReleaseMode::Press);
                    continue;
                }
                if upper == "RELEASE" {
                    press_release_mode = Some(PressReleaseMode::Release);
                    continue;
                }
                if upper == "CLEAR" {
                    pending_modifiers.clear();
                    pending_hold = None;
                    press_release_mode = None;
                    continue;
                }

                // Wait tags
                if let Some(arg) = upper.strip_prefix("WAITMS:") {
                    let ms = parse_wait_ms_value(arg, false, &mut notes)?;
                    t = t.saturating_add(ms).min(MAX_MS);
                    continue;
                }
                if let Some(arg) = upper
                    .strip_prefix("WAITMS")
                    .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
                {
                    let ms = parse_wait_ms_value(arg, false, &mut notes)?;
                    t = t.saturating_add(ms).min(MAX_MS);
                    continue;
                }
                if let Some(arg) = upper.strip_prefix("WAIT:") {
                    let ms = parse_wait_ms_value(arg, true, &mut notes)?;
                    t = t.saturating_add(ms).min(MAX_MS);
                    continue;
                }

                // Hold tags
                if let Some(arg) = upper.strip_prefix("HOLDMS:") {
                    let ms = parse_wait_ms_value(arg, false, &mut notes)?.max(1);
                    pending_hold = Some(ms);
                    continue;
                }
                if let Some(arg) = upper
                    .strip_prefix("HOLDMS")
                    .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
                {
                    let ms = parse_wait_ms_value(arg, false, &mut notes)?.max(1);
                    pending_hold = Some(ms);
                    continue;
                }
                if let Some(arg) = upper.strip_prefix("HOLD:") {
                    let ms = parse_wait_ms_value(arg, true, &mut notes)?.max(1);
                    pending_hold = Some(ms);
                    continue;
                }

                // Modifiers
                if let Some(mod_key) = parse_modifier_tag(&upper) {
                    if press_release_mode.is_some() {
                        t = emit_target(
                            InputSource::Keyboard(mod_key),
                            t,
                            &mut pending_modifiers,
                            &mut pending_hold,
                            &mut press_release_mode,
                            &mut open_holds,
                            &mut raw_events,
                        )?;
                    } else if !pending_modifiers.contains(&mod_key) {
                        pending_modifiers.push(mod_key);
                    }
                    continue;
                }

                // Mouse Down / Up explicit tags
                let mouse_down = match upper.as_str() {
                    "LMBD" => Some(MouseButton::Left),
                    "RMBD" => Some(MouseButton::Right),
                    "MMBD" => Some(MouseButton::Middle),
                    "MB4D" | "XMB1D" => Some(MouseButton::XButton1),
                    "MB5D" | "XMB2D" => Some(MouseButton::XButton2),
                    _ => None,
                };
                if let Some(btn) = mouse_down {
                    let mut mode = Some(PressReleaseMode::Press);
                    t = emit_target(
                        InputSource::MouseButton(btn),
                        t,
                        &mut pending_modifiers,
                        &mut pending_hold,
                        &mut mode,
                        &mut open_holds,
                        &mut raw_events,
                    )?;
                    continue;
                }

                let mouse_up = match upper.as_str() {
                    "LMBU" => Some(MouseButton::Left),
                    "RMBU" => Some(MouseButton::Right),
                    "MMBU" => Some(MouseButton::Middle),
                    "MB4U" | "XMB1U" => Some(MouseButton::XButton1),
                    "MB5U" | "XMB2U" => Some(MouseButton::XButton2),
                    _ => None,
                };
                if let Some(btn) = mouse_up {
                    let mut mode = Some(PressReleaseMode::Release);
                    t = emit_target(
                        InputSource::MouseButton(btn),
                        t,
                        &mut pending_modifiers,
                        &mut pending_hold,
                        &mut mode,
                        &mut open_holds,
                        &mut raw_events,
                    )?;
                    continue;
                }

                // Mouse Tap tags
                let mouse_tap = match upper.as_str() {
                    "LMB" => Some(MouseButton::Left),
                    "RMB" => Some(MouseButton::Right),
                    "MMB" => Some(MouseButton::Middle),
                    "MB4" | "XMB1" | "MB4/XMB1" => Some(MouseButton::XButton1),
                    "MB5" | "XMB2" | "MB5/XMB2" => Some(MouseButton::XButton2),
                    _ => None,
                };
                if let Some(btn) = mouse_tap {
                    t = emit_target(
                        InputSource::MouseButton(btn),
                        t,
                        &mut pending_modifiers,
                        &mut pending_hold,
                        &mut press_release_mode,
                        &mut open_holds,
                        &mut raw_events,
                    )?;
                    continue;
                }

                // Low-level scan code / virtual key tags
                if let Some(arg) = upper.strip_prefix("SC:") {
                    let code = parse_number_u64(arg)
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or_else(|| format!("Scan code không hợp lệ trong '{{{tag_raw}}}'"))?;
                    let key = KeyboardKey::OemScanCode(ScanCode::new(code, false));
                    t = emit_target(
                        InputSource::Keyboard(key),
                        t,
                        &mut pending_modifiers,
                        &mut pending_hold,
                        &mut press_release_mode,
                        &mut open_holds,
                        &mut raw_events,
                    )?;
                    continue;
                }
                if let Some(arg) = upper.strip_prefix("SCE:") {
                    let code = parse_number_u64(arg)
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or_else(|| format!("Scan code không hợp lệ trong '{{{tag_raw}}}'"))?;
                    let key = KeyboardKey::OemScanCode(ScanCode::new(code, true));
                    t = emit_target(
                        InputSource::Keyboard(key),
                        t,
                        &mut pending_modifiers,
                        &mut pending_hold,
                        &mut press_release_mode,
                        &mut open_holds,
                        &mut raw_events,
                    )?;
                    continue;
                }
                if let Some(arg) = upper
                    .strip_prefix("VKC:")
                    .or_else(|| upper.strip_prefix("EXT:"))
                {
                    let code = parse_number_u64(arg)
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or_else(|| format!("Mã phím không hợp lệ trong '{{{tag_raw}}}'"))?;
                    let key =
                        KeyboardKey::OemScanCode(ScanCode::new(code, upper.starts_with("EXT:")));
                    t = emit_target(
                        InputSource::Keyboard(key),
                        t,
                        &mut pending_modifiers,
                        &mut pending_hold,
                        &mut press_release_mode,
                        &mut open_holds,
                        &mut raw_events,
                    )?;
                    continue;
                }

                // Named special keys
                if let Some(named_key) = parse_named_key_tag(&upper) {
                    t = emit_target(
                        InputSource::Keyboard(named_key),
                        t,
                        &mut pending_modifiers,
                        &mut pending_hold,
                        &mut press_release_mode,
                        &mut open_holds,
                        &mut raw_events,
                    )?;
                    continue;
                }

                // System tags (syntactically valid, no timeline clip created)
                if is_system_tag(&upper) {
                    system_tags.push(format!("{{{tag_raw}}}"));
                    continue;
                }

                return Err(format!("Thẻ lệnh không xác định: '{{{tag_raw}}}'"));
            }
        }
    }

    if let Some(mode) = press_release_mode {
        let tag = match mode {
            PressReleaseMode::Press => "{PRESS}",
            PressReleaseMode::Release => "{RELEASE}",
        };
        return Err(format!("Thẻ {tag} ở cuối chuỗi chưa có phím theo sau."));
    }
    if !pending_modifiers.is_empty() {
        let names: Vec<String> = pending_modifiers
            .iter()
            .map(|&k| keyboard_key_short_label(k))
            .collect();
        return Err(format!(
            "Modifier [{}] ở cuối chuỗi chưa có phím chính theo sau.",
            names.join(", ")
        ));
    }
    if let Some(ms) = pending_hold {
        return Err(format!(
            "Thẻ {{HOLDMS:{ms}}} ở cuối chuỗi chưa có phím hoặc nút chuột theo sau."
        ));
    }
    if !open_holds.is_empty() {
        let mut names: Vec<String> = open_holds.keys().map(|s| s.display_name()).collect();
        names.sort();
        return Err(format!(
            "Phím/chuột [{}] đang nhấn giữ ({{PRESS}} / *D) nhưng thiếu {{RELEASE}} / *U đóng lại.",
            names.join(", ")
        ));
    }

    raw_events.sort_by(|a, b| {
        a.clip
            .start_ms
            .cmp(&b.clip.start_ms)
            .then_with(|| a.order.cmp(&b.order))
            .then_with(|| b.clip.dur_ms.cmp(&a.clip.dur_ms))
    });

    let mut clips: Vec<Clip> = raw_events.into_iter().map(|ev| ev.clip).collect();
    let max_concurrent_tracks = if clips.is_empty() {
        0
    } else {
        allocate_tracks_greedy(&mut clips);
        clips.iter().map(|c| c.track + 1).max().unwrap_or(1)
    };
    let total_ms = clips.iter().map(Clip::end_ms).max().unwrap_or(0).max(t);

    if DEFAULT_CLIP_MS == 0 {
        unreachable!();
    }

    Ok(ParsedMacroSequence {
        clips,
        max_concurrent_tracks,
        total_ms,
        notes,
        system_tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_export_round_trip_with_hold_group_and_parallel_press_release() {
        let seq = "{HOLDMS:120}{CTRL}c{WAITMS:50}{LMBD}{WAITMS:40}{PRESS}e{WAITMS:80}{RELEASE}e{WAITMS:30}{LMBU}";
        let mut next_id = 1;
        let parsed = parse_macro_sequence(seq, DEFAULT_CLIP_MS, &mut next_id)
            .expect("Valid sequence should parse cleanly");

        assert_eq!(parsed.clips.len(), 4);
        assert_eq!(parsed.max_concurrent_tracks, 2);
        assert_eq!(parsed.total_ms, 320);

        let exported = export_clips_to_macro_text(&parsed.clips);
        let mut next_id_2 = 100;
        let reparsed = parse_macro_sequence(&exported, DEFAULT_CLIP_MS, &mut next_id_2)
            .expect("Exported sequence must re-parse cleanly");
        assert_eq!(reparsed.clips.len(), parsed.clips.len());
        assert_eq!(reparsed.total_ms, parsed.total_ms);
        assert!(exported.contains("{HOLDMS:120}{CTRL}c"));
    }

    #[test]
    fn parser_supports_special_delete_wait_ranges_and_system_tags() {
        let mut next_id = 1;
        let parsed =
            parse_macro_sequence("{CTRL}{ALT}DELETE{WAITMS:40-60}{MWUP}", 100, &mut next_id)
                .expect("Sequence with literal DELETE, wait range, and MWUP should succeed");
        assert_eq!(parsed.clips.len(), 3);
        assert_eq!(parsed.total_ms, 150);
        assert_eq!(parsed.notes.len(), 1);
        assert_eq!(parsed.system_tags, vec!["{MWUP}"]);
    }

    #[test]
    fn parser_rejects_unbalanced_press_and_syntax_errors() {
        let mut next_id = 1;
        assert!(parse_macro_sequence("{PRESS}e{WAITMS:50}", 100, &mut next_id).is_err());
        assert!(parse_macro_sequence("{RELEASE}e", 100, &mut next_id).is_err());
        assert!(parse_macro_sequence("{LMBD}{WAITMS:50}", 100, &mut next_id).is_err());
        assert!(parse_macro_sequence("{WAITMS:50", 100, &mut next_id).is_err());
        assert!(parse_macro_sequence("{}", 100, &mut next_id).is_err());
        assert!(parse_macro_sequence("{CTRL}", 100, &mut next_id).is_err());
        assert!(parse_macro_sequence("{HOLDMS:100}", 100, &mut next_id).is_err());
    }
}
