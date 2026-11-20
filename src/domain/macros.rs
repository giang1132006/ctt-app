//! Pure domain models, NLE timeline algorithms, compiler, and validation for macros.
//!
//! This module is intentionally decoupled from UI (`slint`), OS hooks (`platform`),
//! and persistence (`storage`) so all macro timeline logic remains deterministically testable.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::domain::config::{EmergencyStopHotkey, CURRENT_SCHEMA_VERSION};
use crate::domain::keys::{InputSource, KeyboardKey, MouseButton};

/// Maximum supported macro timeline length in milliseconds (10 minutes).
pub const MAX_MS: u64 = 600_000;
/// Minimum number of tracks always displayed in the NLE timeline.
pub const MIN_TRACKS: usize = 4;
/// Maximum number of undo/redo snapshots retained in editor history.
pub const HISTORY_LIMIT: usize = 100;
/// Default duration in milliseconds when inserting a new clip.
pub const DEFAULT_CLIP_MS: u64 = 100;

/// Width in logical pixels of the sticky left track header column (`T1..Tn`).
pub const HEAD_W: f32 = 58.0;
/// Height in logical pixels of the sticky top time ruler.
pub const RULER_H: f32 = 28.0;
/// Height in logical pixels of a single track lane.
pub const TRACK_H: f32 = 38.0;
/// Pixel distance from clip left/right edge that triggers horizontal trim resizing.
pub const EDGE_PX: f32 = 6.0;
/// Pixel threshold for magnetic snapping.
pub const SNAP_PX: f32 = 8.0;
/// Minimum zoom factor (`px / ms`).
pub const ZOOM_MIN: f32 = 0.02;
/// Maximum zoom factor (`px / ms`).
pub const ZOOM_MAX: f32 = 6.0;
/// Default zoom factor (`0.5 px / ms` = `200 ms / 100 px`).
pub const DEFAULT_ZOOM: f32 = 0.5;
/// Pointer drag threshold in pixels before entering drag/marquee motion.
pub const DRAG_THRESHOLD: f32 = 3.0;

/// Stable identifier referencing a saved macro definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacroId(pub u32);

/// Playback execution behavior for macro triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MacroPlaybackMode {
    RunOnce,
    RepeatCount(u32),
    ToggleLoop,
    HoldLoop,
}

impl MacroPlaybackMode {
    /// Human-readable label for UI display.
    pub fn display_name(self) -> String {
        match self {
            Self::RunOnce => "Chạy 1 lần (Once)".to_string(),
            Self::RepeatCount(n) => format!("Lặp {n} lần"),
            Self::ToggleLoop => "Bật/Tắt lặp (Toggle)".to_string(),
            Self::HoldLoop => "Giữ để lặp (Hold)".to_string(),
        }
    }

    /// Stable serialization token.
    pub fn short_code(self) -> String {
        match self {
            Self::RunOnce => "ONCE".to_string(),
            Self::RepeatCount(n) => format!("REPEAT:{n}"),
            Self::ToggleLoop => "TOGGLE".to_string(),
            Self::HoldLoop => "HOLD".to_string(),
        }
    }

    /// Parses a [`MacroPlaybackMode`] from its serialization token.
    pub fn from_short_code(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("ONCE") {
            return Some(Self::RunOnce);
        }
        if trimmed.eq_ignore_ascii_case("TOGGLE") {
            return Some(Self::ToggleLoop);
        }
        if trimmed.eq_ignore_ascii_case("HOLD") {
            return Some(Self::HoldLoop);
        }
        if let Some(count_str) = trimmed.strip_prefix("REPEAT:") {
            let count = count_str.parse::<u32>().ok()?.max(1);
            return Some(Self::RepeatCount(count));
        }
        None
    }

    /// Maps the playback mode to the UI selector index (`0 = Hold`, `1 = Toggle`, `2 = Once`).
    pub const fn to_ui_index(self) -> i32 {
        match self {
            Self::HoldLoop => 0,
            Self::ToggleLoop => 1,
            Self::RunOnce | Self::RepeatCount(_) => 2,
        }
    }

    /// Maps a UI selector index back to [`MacroPlaybackMode`].
    pub const fn from_ui_index(idx: i32) -> Self {
        match idx {
            1 => Self::ToggleLoop,
            2 => Self::RunOnce,
            _ => Self::HoldLoop,
        }
    }
}

/// Kind of input action represented by a timeline clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipKind {
    /// Keyboard press/hold clip; `None` indicates a newly placed `?` clip awaiting key capture.
    Key { key: Option<KeyboardKey> },
    /// Mouse button press/hold clip.
    Mouse { button: MouseButton },
}

/// A single timed input-hold clip on a track in the NLE multi-track timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    pub id: u32,
    pub kind: ClipKind,
    pub track: usize,
    pub start_ms: u64,
    pub dur_ms: u64,
}

impl Clip {
    /// Returns the exclusive release timestamp (`start_ms + dur_ms`).
    pub const fn end_ms(&self) -> u64 {
        self.start_ms.saturating_add(self.dur_ms)
    }

    /// Returns `true` if this clip is a keyboard clip that does not yet have a key assigned.
    pub const fn is_unassigned_key(&self) -> bool {
        matches!(self.kind, ClipKind::Key { key: None })
    }

    /// Returns a compact label suitable for drawing inside the timeline clip rectangle.
    pub fn short_label(&self) -> String {
        match self.kind {
            ClipKind::Key { key: Some(key) } => keyboard_key_short_label(key),
            ClipKind::Key { key: None } => "?".to_string(),
            ClipKind::Mouse { button } => mouse_button_short_label(button).to_string(),
        }
    }

    /// Returns a descriptive label for inspector headers and validation errors.
    pub fn full_label(&self) -> String {
        match self.kind {
            ClipKind::Key { key: Some(key) } => key.display_name(),
            ClipKind::Key { key: None } => "Phím chưa gán (?)".to_string(),
            ClipKind::Mouse { button } => mouse_button_full_label(button).to_string(),
        }
    }

    /// Returns a canonical physical target key for detecting simultaneous overlapping holds.
    pub fn target_id(&self) -> Option<InputSource> {
        match self.kind {
            ClipKind::Key { key: Some(key) } => Some(InputSource::Keyboard(key)),
            ClipKind::Key { key: None } => None,
            ClipKind::Mouse { button } => Some(InputSource::MouseButton(button)),
        }
    }
}

/// Returns the compact timeline label (`LC`, `RC`, `MC`, `M3`, `M4`) for a mouse button.
pub const fn mouse_button_short_label(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "LC",
        MouseButton::Right => "RC",
        MouseButton::Middle => "MC",
        MouseButton::XButton1 => "M3",
        MouseButton::XButton2 => "M4",
    }
}

/// Returns the descriptive Vietnamese label for a mouse button.
pub const fn mouse_button_full_label(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "Chuột trái (LC)",
        MouseButton::Right => "Chuột phải (RC)",
        MouseButton::Middle => "Chuột giữa (MC)",
        MouseButton::XButton1 => "Chuột phụ 1 (M3)",
        MouseButton::XButton2 => "Chuột phụ 2 (M4)",
    }
}

/// Returns a compact label for a [`KeyboardKey`] inside narrow timeline clips.
pub fn keyboard_key_short_label(key: KeyboardKey) -> String {
    match key {
        KeyboardKey::Escape => "Esc".to_string(),
        KeyboardKey::Backspace => "Bksp".to_string(),
        KeyboardKey::CapsLock => "Caps".to_string(),
        KeyboardKey::LeftShift => "LShift".to_string(),
        KeyboardKey::RightShift => "RShift".to_string(),
        KeyboardKey::LeftCtrl => "LCtrl".to_string(),
        KeyboardKey::RightCtrl => "RCtrl".to_string(),
        KeyboardKey::LeftAlt => "LAlt".to_string(),
        KeyboardKey::RightAlt => "RAlt".to_string(),
        KeyboardKey::LeftMeta => "LWin".to_string(),
        KeyboardKey::RightMeta => "RWin".to_string(),
        KeyboardKey::PrintScreen => "PrtSc".to_string(),
        KeyboardKey::ScrollLock => "ScrLk".to_string(),
        KeyboardKey::PageUp => "PgUp".to_string(),
        KeyboardKey::PageDown => "PgDn".to_string(),
        KeyboardKey::Insert => "Ins".to_string(),
        KeyboardKey::Delete => "Del".to_string(),
        KeyboardKey::ArrowUp => "↑".to_string(),
        KeyboardKey::ArrowDown => "↓".to_string(),
        KeyboardKey::ArrowLeft => "←".to_string(),
        KeyboardKey::ArrowRight => "→".to_string(),
        KeyboardKey::NumLock => "NumLk".to_string(),
        KeyboardKey::NumpadDivide => "Num/".to_string(),
        KeyboardKey::NumpadMultiply => "Num*".to_string(),
        KeyboardKey::NumpadSubtract => "Num-".to_string(),
        KeyboardKey::NumpadAdd => "Num+".to_string(),
        KeyboardKey::NumpadEnter => "Num↵".to_string(),
        KeyboardKey::NumpadDecimal => "Num.".to_string(),
        KeyboardKey::Numpad0 => "Num0".to_string(),
        KeyboardKey::Numpad1 => "Num1".to_string(),
        KeyboardKey::Numpad2 => "Num2".to_string(),
        KeyboardKey::Numpad3 => "Num3".to_string(),
        KeyboardKey::Numpad4 => "Num4".to_string(),
        KeyboardKey::Numpad5 => "Num5".to_string(),
        KeyboardKey::Numpad6 => "Num6".to_string(),
        KeyboardKey::Numpad7 => "Num7".to_string(),
        KeyboardKey::Numpad8 => "Num8".to_string(),
        KeyboardKey::Numpad9 => "Num9".to_string(),
        KeyboardKey::VolumeMute => "Mute".to_string(),
        KeyboardKey::VolumeDown => "Vol-".to_string(),
        KeyboardKey::VolumeUp => "Vol+".to_string(),
        KeyboardKey::MediaPlayPause => "Play".to_string(),
        KeyboardKey::MediaStop => "Stop".to_string(),
        KeyboardKey::MediaNext => "Next".to_string(),
        KeyboardKey::MediaPrev => "Prev".to_string(),
        other => other.display_name(),
    }
}

/// Loop interval `[start_ms, end_ms]` configured on the NLE timeline ruler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoopRegion {
    pub start_ms: u64,
    pub end_ms: u64,
}

/// Serializable state snapshot used for 100-step Undo/Redo history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineSnapshot {
    pub clips: Vec<Clip>,
    pub tracks: usize,
    pub loop_region: Option<LoopRegion>,
}

/// Flat sequential step produced by compiling the multi-track NLE timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MacroStep {
    KeyDown(KeyboardKey),
    KeyUp(KeyboardKey),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    Wait(u64),
    LoopStart,
    LoopEnd,
}

impl fmt::Display for MacroStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyDown(k) => write!(f, "KeyDown({})", k.display_name()),
            Self::KeyUp(k) => write!(f, "KeyUp({})", k.display_name()),
            Self::MouseDown(b) => write!(f, "MouseDown({})", mouse_button_short_label(*b)),
            Self::MouseUp(b) => write!(f, "MouseUp({})", mouse_button_short_label(*b)),
            Self::Wait(ms) => write!(f, "Wait({ms} ms)"),
            Self::LoopStart => f.write_str("LoopStart"),
            Self::LoopEnd => f.write_str("LoopEnd"),
        }
    }
}

/// Optional hotkey trigger binding for activating a saved macro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacroTrigger {
    pub source: InputSource,
    pub mode: MacroPlaybackMode,
}

/// Complete saved macro definition stored inside a [`crate::domain::profile::Profile`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroDefinition {
    pub id: MacroId,
    pub name: String,
    pub schema_version: u32,
    pub enabled: bool,
    pub trigger: Option<MacroTrigger>,
    pub clips: Vec<Clip>,
    pub tracks: usize,
    pub loop_region: Option<LoopRegion>,
    pub compiled_steps: Vec<MacroStep>,
}

impl MacroDefinition {
    /// Creates a validated macro definition and computes its compiled execution steps.
    pub fn new(
        id: MacroId,
        name: impl Into<String>,
        trigger: Option<MacroTrigger>,
        clips: Vec<Clip>,
        tracks: usize,
        loop_region: Option<LoopRegion>,
    ) -> Self {
        let compiled_steps = compile_clips(&clips, loop_region);
        Self {
            id,
            name: name.into(),
            schema_version: CURRENT_SCHEMA_VERSION,
            enabled: true,
            trigger,
            clips,
            tracks: tracks.max(MIN_TRACKS),
            loop_region,
            compiled_steps,
        }
    }
}

/// Computes the major tick spacing in milliseconds so major ruler labels stay >= 90 px apart.
pub fn ruler_step(zoom: f32) -> u64 {
    const STEPS: [u64; 15] = [
        1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 30000, 60000,
    ];
    let z = zoom.max(ZOOM_MIN);
    STEPS
        .into_iter()
        .find(|&step| (step as f32) * z >= 90.0)
        .unwrap_or(60_000)
}

/// Formats a major ruler tick label (`ms` or compact seconds like `1.5s` when `major_step >= 1000`).
pub fn format_ruler_ms(ms: u64, major_step: u64) -> String {
    if major_step >= 1000 {
        let secs = ms as f64 / 1000.0;
        let formatted = format!("{secs:.2}");
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
        format!("{trimmed}s")
    } else {
        ms.to_string()
    }
}

/// Returns `true` when two half-open intervals `[a_start, a_start + a_dur)` and `[b_start, b_start + b_dur)` overlap.
pub const fn overlaps(a_start: u64, a_dur: u64, b_start: u64, b_dur: u64) -> bool {
    let a_end = a_start.saturating_add(a_dur);
    let b_end = b_start.saturating_add(b_dur);
    a_start < b_end && b_start < a_end
}

/// Collects magnetic snap anchor timestamps (0, optional playhead, clip edges, and loop bounds).
pub fn collect_snap_points(
    clips: &[Clip],
    playhead_ms: u64,
    loop_region: Option<LoopRegion>,
    ignore: &HashSet<u32>,
    include_playhead: bool,
) -> Vec<u64> {
    let mut pts = Vec::with_capacity(clips.len() * 2 + 4);
    pts.push(0);
    if include_playhead {
        pts.push(playhead_ms);
    }
    for c in clips {
        if !ignore.contains(&c.id) {
            pts.push(c.start_ms);
            pts.push(c.end_ms());
        }
    }
    if let Some(lp) = loop_region {
        pts.push(lp.start_ms);
        pts.push(lp.end_ms);
    }
    pts
}

/// Snaps a single timestamp `value_ms` to the nearest anchor within `SNAP_PX / zoom`.
pub fn snap_time(
    clips: &[Clip],
    playhead_ms: u64,
    loop_region: Option<LoopRegion>,
    zoom: f32,
    value_ms: f64,
    disabled: bool,
    ignore: &HashSet<u32>,
) -> (f64, Option<u64>) {
    if disabled {
        return (value_ms, None);
    }
    let threshold = (SNAP_PX / zoom.max(ZOOM_MIN)) as f64;
    let mut best: Option<u64> = None;
    for pt in collect_snap_points(clips, playhead_ms, loop_region, ignore, true) {
        let dist = (pt as f64 - value_ms).abs();
        if dist <= threshold && best.is_none_or(|b| dist < (b as f64 - value_ms).abs()) {
            best = Some(pt);
        }
    }
    match best {
        Some(pt) => (pt as f64, Some(pt)),
        None => (value_ms, None),
    }
}

/// Snaps a group shift (`shift_ms`) by checking both start and end edges of every clip in `origs`.
#[allow(clippy::too_many_arguments)]
pub fn snap_shift(
    clips: &[Clip],
    playhead_ms: u64,
    loop_region: Option<LoopRegion>,
    zoom: f32,
    origs: &[(u64, u64)],
    shift_ms: f64,
    disabled: bool,
    ignore: &HashSet<u32>,
) -> (f64, Option<u64>) {
    if disabled {
        return (shift_ms, None);
    }
    let threshold = (SNAP_PX / zoom.max(ZOOM_MIN)) as f64;
    let points = collect_snap_points(clips, playhead_ms, loop_region, ignore, true);
    let mut best: Option<(f64, u64)> = None;
    for &(orig_start, orig_dur) in origs {
        for edge in [orig_start, orig_start.saturating_add(orig_dur)] {
            for &pt in &points {
                let delta = pt as f64 - (edge as f64 + shift_ms);
                if delta.abs() <= threshold && best.is_none_or(|(bd, _)| delta.abs() < bd.abs()) {
                    best = Some((delta, pt));
                }
            }
        }
    }
    match best {
        Some((delta, at)) => (shift_ms + delta, Some(at)),
        None => (shift_ms, None),
    }
}

/// Snaps a playhead click/scrub position to clip/loop edges (excluding the old playhead) within `0..=MAX_MS`.
pub fn snap_playhead(
    clips: &[Clip],
    loop_region: Option<LoopRegion>,
    zoom: f32,
    value_ms: f64,
    disabled: bool,
) -> u64 {
    let clamped = value_ms.clamp(0.0, MAX_MS as f64);
    if disabled {
        return clamped.round() as u64;
    }
    let threshold = (SNAP_PX / zoom.max(ZOOM_MIN)) as f64;
    let empty_ignore = HashSet::new();
    let mut best: Option<u64> = None;
    for pt in collect_snap_points(clips, 0, loop_region, &empty_ignore, false) {
        let dist = (pt as f64 - clamped).abs();
        if dist <= threshold && best.is_none_or(|b| dist < (b as f64 - clamped).abs()) {
            best = Some(pt);
        }
    }
    best.unwrap_or_else(|| clamped.round() as u64).min(MAX_MS)
}

/// Finds the first non-overlapping track >= `preferred` (or wrapping below if occupied).
pub fn free_track_at(
    clips: &[Clip],
    start_ms: u64,
    dur_ms: u64,
    preferred: usize,
    ignore: &HashSet<u32>,
) -> usize {
    let has_clash = |track: usize| {
        clips.iter().any(|c| {
            !ignore.contains(&c.id)
                && c.track == track
                && overlaps(c.start_ms, c.dur_ms, start_ms, dur_ms)
        })
    };
    let mut track = preferred;
    while has_clash(track) {
        track += 1;
    }
    track
}

/// Resolves same-track overlaps for `moved_ids` by moving colliding clips to the nearest free track.
pub fn resolve_overlaps(clips: &mut [Clip], moved_ids: &HashSet<u32>, direction: isize) {
    let ids: Vec<u32> = clips
        .iter()
        .filter(|c| moved_ids.contains(&c.id))
        .map(|c| c.id)
        .collect();

    for id in ids {
        let Some(idx) = clips.iter().position(|c| c.id == id) else {
            continue;
        };
        let (start_ms, dur_ms, cur_track) =
            (clips[idx].start_ms, clips[idx].dur_ms, clips[idx].track);

        let has_clash = |track: usize, list: &[Clip]| -> bool {
            list.iter().enumerate().any(|(j, other)| {
                j != idx
                    && other.track == track
                    && overlaps(other.start_ms, other.dur_ms, start_ms, dur_ms)
            })
        };

        if !has_clash(cur_track, clips) {
            continue;
        }

        let mut placed = false;
        if direction < 0 && cur_track > 0 {
            for t in (0..cur_track).rev() {
                if !has_clash(t, clips) {
                    clips[idx].track = t;
                    placed = true;
                    break;
                }
            }
        }
        if !placed {
            let mut t = cur_track + 1;
            while has_clash(t, clips) {
                t += 1;
            }
            clips[idx].track = t;
        }
    }
}

/// Inserts a blank time gap of `gap_ms` at `playhead_ms`.
///
/// Clips starting at or after `playhead_ms` are shifted right; clips spanning `playhead_ms`
/// are stretched by `gap_ms`. Loop boundaries `>= playhead_ms` shift right as well.
pub fn insert_time_gap(
    clips: &mut [Clip],
    loop_region: &mut Option<LoopRegion>,
    playhead_ms: u64,
    gap_ms: u64,
) {
    if gap_ms == 0 {
        return;
    }
    for clip in clips.iter_mut() {
        if clip.start_ms >= playhead_ms {
            clip.start_ms = clip.start_ms.saturating_add(gap_ms).min(MAX_MS);
            if clip.end_ms() > MAX_MS {
                clip.dur_ms = MAX_MS.saturating_sub(clip.start_ms).max(1);
            }
        } else if clip.start_ms < playhead_ms && clip.end_ms() > playhead_ms {
            let new_dur = clip.dur_ms.saturating_add(gap_ms);
            clip.dur_ms = new_dur.min(MAX_MS.saturating_sub(clip.start_ms)).max(1);
        }
    }
    if let Some(lp) = loop_region.as_mut() {
        if lp.start_ms >= playhead_ms {
            lp.start_ms = lp.start_ms.saturating_add(gap_ms).min(MAX_MS);
        }
        if lp.end_ms >= playhead_ms {
            lp.end_ms = lp.end_ms.saturating_add(gap_ms).min(MAX_MS);
        }
    }
}

/// Splits all targeted clips spanning `playhead_ms` (`start < playhead < end`) into two adjacent clips.
///
/// If `selected_ids` is non-empty, only selected clips spanning `playhead_ms` are split;
/// otherwise all clips spanning `playhead_ms` are split. Returns IDs of the newly created right halves.
pub fn razor_split_at(
    clips: &mut Vec<Clip>,
    selected_ids: &HashSet<u32>,
    playhead_ms: u64,
    next_id: &mut u32,
) -> Vec<u32> {
    let filter_selected = !selected_ids.is_empty();
    let mut newly_created = Vec::new();
    let mut additions = Vec::new();

    for clip in clips.iter_mut() {
        if filter_selected && !selected_ids.contains(&clip.id) {
            continue;
        }
        if clip.start_ms < playhead_ms && clip.end_ms() > playhead_ms {
            let old_end = clip.end_ms();
            clip.dur_ms = playhead_ms - clip.start_ms;
            let new_id = *next_id;
            *next_id += 1;
            newly_created.push(new_id);
            additions.push(Clip {
                id: new_id,
                kind: clip.kind,
                track: clip.track,
                start_ms: playhead_ms,
                dur_ms: old_end - playhead_ms,
            });
        }
    }

    clips.extend(additions);
    newly_created
}

/// Detects clips that overlap in time while targeting the exact same physical keyboard key or mouse button.
pub fn clashing_ids(clips: &[Clip]) -> HashSet<u32> {
    let mut by_target: HashMap<InputSource, Vec<&Clip>> = HashMap::new();
    for clip in clips {
        if let Some(target) = clip.target_id() {
            by_target.entry(target).or_default().push(clip);
        }
    }

    let mut clashes = HashSet::new();
    for group in by_target.values() {
        for i in 0..group.len() {
            for j in (i + 1)..group.len() {
                if overlaps(
                    group[i].start_ms,
                    group[i].dur_ms,
                    group[j].start_ms,
                    group[j].dur_ms,
                ) {
                    clashes.insert(group[i].id);
                    clashes.insert(group[j].id);
                }
            }
        }
    }
    clashes
}

/// Validates a multi-track macro timeline before saving.
///
/// Enforces all 6 safety and structural invariants:
/// 1. Valid non-empty name (`<= 80` chars) and non-empty timeline.
/// 2. No unassigned `?` keyboard clips.
/// 3. Every clip has `dur_ms >= 1`.
/// 4. Zero simultaneous overlapping holds of the same physical key/button (`clashing_ids`).
/// 5. Loop region contains at least one full clip, does not bisect any clip across `start`/`end`, and leaves no unreachable clips after `end`.
/// 6. Trigger hotkey cannot be Left/Right mouse button, cannot be the Emergency Stop hotkey, and cannot self-trigger from inside the macro.
pub fn validate_macro_timeline(
    name: &str,
    hotkey: Option<InputSource>,
    clips: &[Clip],
    loop_region: Option<LoopRegion>,
    emergency_stop: EmergencyStopHotkey,
) -> Result<(), String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Vui lòng nhập Tên Macro trước khi lưu.".to_string());
    }
    if trimmed_name.chars().count() > 80 {
        return Err("Tên Macro không được vượt quá 80 ký tự.".to_string());
    }
    if clips.is_empty() {
        return Err("Timeline đang trống — hãy thêm ít nhất 1 clip phím hoặc chuột.".to_string());
    }

    for clip in clips {
        if clip.is_unassigned_key() {
            return Err(format!(
                "Clip ? ở {} ms (T{}) chưa chọn phím. Nhấp đúp vào clip để chọn.",
                clip.start_ms,
                clip.track + 1
            ));
        }
        if clip.dur_ms < 1 {
            return Err(format!(
                "Clip {} ở {} ms: thời gian giữ phải từ 1 ms.",
                clip.full_label(),
                clip.start_ms
            ));
        }
    }

    let clashes = clashing_ids(clips);
    if let Some(first_id) = clashes.iter().min().copied() {
        let label = clips
            .iter()
            .find(|c| c.id == first_id)
            .map(Clip::full_label)
            .unwrap_or_else(|| "Clip".to_string());
        return Err(format!(
            "{label} bị giữ chồng lên chính nó ở hai track (viền đỏ). Dời để hai clip không trùng thời gian."
        ));
    }

    if let Some(lp) = loop_region {
        if lp.end_ms <= lp.start_ms {
            return Err("Vùng lặp không hợp lệ: điểm cuối phải lớn hơn điểm đầu.".to_string());
        }
        for clip in clips {
            if clip.start_ms < lp.start_ms && clip.end_ms() > lp.start_ms {
                return Err(format!(
                    "Clip {} cắt ngang mép đầu vùng lặp ({} ms). Hãy bấm C để cắt đôi hoặc dời ra ngoài/vào trong.",
                    clip.short_label(),
                    lp.start_ms
                ));
            }
            if clip.start_ms < lp.end_ms && clip.end_ms() > lp.end_ms {
                return Err(format!(
                    "Clip {} cắt ngang mép cuối vùng lặp ({} ms). Hãy bấm C để cắt đôi hoặc dời vào trong/ra ngoài.",
                    clip.short_label(),
                    lp.end_ms
                ));
            }
            if clip.start_ms >= lp.end_ms {
                return Err(format!(
                    "Clip {} ở {} ms nằm sau điểm cuối vùng lặp ({} ms) nên sẽ không bao giờ chạy.",
                    clip.short_label(),
                    clip.start_ms,
                    lp.end_ms
                ));
            }
        }
        let has_clip_inside = clips
            .iter()
            .any(|c| c.start_ms >= lp.start_ms && c.end_ms() <= lp.end_ms);
        if !has_clip_inside {
            return Err(
                "Vùng lặp đang trống, hãy đặt ít nhất 1 clip vào trong vùng lặp.".to_string(),
            );
        }
    }

    if let Some(hk) = hotkey {
        if matches!(
            hk,
            InputSource::MouseButton(MouseButton::Left | MouseButton::Right)
        ) {
            return Err(
                "Không thể gán Chuột trái hoặc Chuột phải làm Hotkey kích hoạt Macro.".to_string(),
            );
        }
        if emergency_stop.modifiers.is_empty()
            && hk == InputSource::Keyboard(emergency_stop.trigger_key)
        {
            return Err(format!(
                "Hotkey '{}' trùng với phím Emergency Stop.",
                hk.display_name()
            ));
        }
        for clip in clips {
            if clip.target_id() == Some(hk) {
                return Err(format!(
                    "Clip {} bấm chính hotkey {} của macro, sẽ tự kích hoạt lại.",
                    clip.short_label(),
                    hk.display_name()
                ));
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct CompileEvent {
    t: u64,
    order: u8,
    track: usize,
    step: MacroStep,
}

impl Ord for CompileEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.t
            .cmp(&other.t)
            .then_with(|| self.order.cmp(&other.order))
            .then_with(|| self.track.cmp(&other.track))
    }
}

impl PartialOrd for CompileEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CompileEvent {
    fn eq(&self, other: &Self) -> bool {
        self.t == other.t && self.order == other.order && self.track == other.track
    }
}

impl Eq for CompileEvent {}

/// Compiles multi-track clips and an optional loop region into flat sequential [`MacroStep`] items.
///
/// Tie-breaking order at identical timestamp `t`:
/// - `order = 0`: KeyUp / MouseUp (`clip.end_ms()`)
/// - `order = 1`: LoopEnd (`loop.end_ms`)
/// - `order = 2`: LoopStart (`loop.start_ms`)
/// - `order = 3`: KeyDown / MouseDown (`clip.start_ms`)
pub fn compile_clips(clips: &[Clip], loop_region: Option<LoopRegion>) -> Vec<MacroStep> {
    let mut events = Vec::with_capacity(clips.len() * 2 + 2);

    for clip in clips {
        match clip.kind {
            ClipKind::Key { key: Some(key) } => {
                events.push(CompileEvent {
                    t: clip.start_ms,
                    order: 3,
                    track: clip.track,
                    step: MacroStep::KeyDown(key),
                });
                events.push(CompileEvent {
                    t: clip.end_ms(),
                    order: 0,
                    track: clip.track,
                    step: MacroStep::KeyUp(key),
                });
            }
            ClipKind::Key { key: None } => {}
            ClipKind::Mouse { button } => {
                events.push(CompileEvent {
                    t: clip.start_ms,
                    order: 3,
                    track: clip.track,
                    step: MacroStep::MouseDown(button),
                });
                events.push(CompileEvent {
                    t: clip.end_ms(),
                    order: 0,
                    track: clip.track,
                    step: MacroStep::MouseUp(button),
                });
            }
        }
    }

    if let Some(lp) = loop_region {
        if lp.end_ms > lp.start_ms {
            events.push(CompileEvent {
                t: lp.start_ms,
                order: 2,
                track: 0,
                step: MacroStep::LoopStart,
            });
            events.push(CompileEvent {
                t: lp.end_ms,
                order: 1,
                track: 0,
                step: MacroStep::LoopEnd,
            });
        }
    }

    events.sort();

    let mut out = Vec::with_capacity(events.len() * 2);
    let mut cursor = 0_u64;
    for ev in events {
        let gap = ev.t.saturating_sub(cursor);
        if gap > 0 {
            out.push(MacroStep::Wait(gap));
        }
        out.push(ev.step);
        cursor = cursor.max(ev.t);
    }

    out
}

/// Reconstructs multi-track clips and an optional loop region from flat [`MacroStep`] items
/// using greedy interval track partitioning.
pub fn steps_to_clips(
    steps: &[MacroStep],
    next_id: &mut u32,
) -> (Vec<Clip>, Option<LoopRegion>, usize) {
    let mut t = 0_u64;
    let mut open: HashMap<InputSource, Clip> = HashMap::new();
    let mut finished: Vec<Clip> = Vec::new();
    let mut loop_start: Option<u64> = None;
    let mut loop_region: Option<LoopRegion> = None;

    for &step in steps {
        match step {
            MacroStep::Wait(ms) => {
                t = t.saturating_add(ms).min(MAX_MS);
            }
            MacroStep::LoopStart => {
                loop_start = Some(t);
            }
            MacroStep::LoopEnd => {
                if let Some(ls) = loop_start.take() {
                    loop_region = Some(LoopRegion {
                        start_ms: ls,
                        end_ms: t.max(ls + 1),
                    });
                }
            }
            MacroStep::KeyDown(key) => {
                let target = InputSource::Keyboard(key);
                if let Some(mut prev) = open.remove(&target) {
                    prev.dur_ms = t.saturating_sub(prev.start_ms).max(1);
                    finished.push(prev);
                }
                let id = *next_id;
                *next_id += 1;
                open.insert(
                    target,
                    Clip {
                        id,
                        kind: ClipKind::Key { key: Some(key) },
                        track: 0,
                        start_ms: t,
                        dur_ms: 1,
                    },
                );
            }
            MacroStep::KeyUp(key) => {
                let target = InputSource::Keyboard(key);
                if let Some(mut clip) = open.remove(&target) {
                    clip.dur_ms = t.saturating_sub(clip.start_ms).max(1);
                    finished.push(clip);
                }
            }
            MacroStep::MouseDown(button) => {
                let target = InputSource::MouseButton(button);
                if let Some(mut prev) = open.remove(&target) {
                    prev.dur_ms = t.saturating_sub(prev.start_ms).max(1);
                    finished.push(prev);
                }
                let id = *next_id;
                *next_id += 1;
                open.insert(
                    target,
                    Clip {
                        id,
                        kind: ClipKind::Mouse { button },
                        track: 0,
                        start_ms: t,
                        dur_ms: 1,
                    },
                );
            }
            MacroStep::MouseUp(button) => {
                let target = InputSource::MouseButton(button);
                if let Some(mut clip) = open.remove(&target) {
                    clip.dur_ms = t.saturating_sub(clip.start_ms).max(1);
                    finished.push(clip);
                }
            }
        }
    }

    for (_, mut clip) in open {
        clip.dur_ms = t.saturating_sub(clip.start_ms).max(1);
        finished.push(clip);
    }

    let tracks = allocate_tracks_greedy(&mut finished);
    (finished, loop_region, tracks.max(MIN_TRACKS))
}

/// Assigns track indices to `clips` using greedy interval partitioning so no two clips share a track if they overlap.
pub fn allocate_tracks_greedy(clips: &mut [Clip]) -> usize {
    clips.sort_by(|a, b| {
        a.start_ms
            .cmp(&b.start_ms)
            .then_with(|| b.dur_ms.cmp(&a.dur_ms))
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut track_ends: Vec<u64> = Vec::new();
    for clip in clips.iter_mut() {
        let mut assigned = None;
        for (idx, end) in track_ends.iter_mut().enumerate() {
            if *end <= clip.start_ms {
                *end = clip.end_ms();
                assigned = Some(idx);
                break;
            }
        }
        if let Some(t) = assigned {
            clip.track = t;
        } else {
            let new_t = track_ends.len();
            track_ends.push(clip.end_ms());
            clip.track = new_t;
        }
    }

    track_ends.len().max(MIN_TRACKS)
}

/// Formats a human-readable execution preview for the UI collapsible preview drawer.
pub fn format_compiled_preview(steps: &[MacroStep]) -> String {
    if steps.is_empty() {
        return "// Timeline trống — chưa có bước thực thi nào.".to_string();
    }
    let mut lines = Vec::with_capacity(steps.len());
    for (idx, step) in steps.iter().enumerate() {
        let num = idx + 1;
        let line = match step {
            MacroStep::KeyDown(k) => format!("{num:>3}. Nhấn giữ phím  [{}]", k.display_name()),
            MacroStep::KeyUp(k) => format!("{num:>3}. Nhả phím       [{}]", k.display_name()),
            MacroStep::MouseDown(b) => {
                format!(
                    "{num:>3}. Nhấn giữ chuột [{}]",
                    mouse_button_short_label(*b)
                )
            }
            MacroStep::MouseUp(b) => {
                format!(
                    "{num:>3}. Nhả chuột      [{}]",
                    mouse_button_short_label(*b)
                )
            }
            MacroStep::Wait(ms) => format!("{num:>3}. Chờ            {ms} ms"),
            MacroStep::LoopStart => format!("{num:>3}. === Bắt đầu vùng lặp ==="),
            MacroStep::LoopEnd => format!("{num:>3}. === Quay lại đầu vùng lặp ==="),
        };
        lines.push(line);
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruler_step_guarantees_minimum_90px_major_spacing() {
        let mut z = ZOOM_MIN;
        while z <= ZOOM_MAX {
            let step = ruler_step(z);
            if step < 60_000 {
                assert!(
                    (step as f32) * z >= 90.0,
                    "step {step} * zoom {z} must be >= 90"
                );
            }
            z *= 1.5;
        }
        assert_eq!(ruler_step(DEFAULT_ZOOM), 200);
        assert_eq!(format_ruler_ms(1500, 1000), "1.5s");
        assert_eq!(format_ruler_ms(200, 200), "200");
    }

    #[test]
    fn snap_and_alt_disable_behave_correctly() {
        let clips = vec![Clip {
            id: 1,
            kind: ClipKind::Key {
                key: Some(KeyboardKey::KeyE),
            },
            track: 0,
            start_ms: 100,
            dur_ms: 150,
        }];
        let ignore = HashSet::new();

        // At zoom = 0.5, threshold is 8 / 0.5 = 16 ms. 244 ms is within 16 ms of 250 ms.
        let (snapped, at) = snap_time(&clips, 0, None, 0.5, 244.0, false, &ignore);
        assert_eq!(snapped, 250.0);
        assert_eq!(at, Some(250));

        // Disabled (Alt held) bypasses snap
        let (unsnapped, at_none) = snap_time(&clips, 0, None, 0.5, 244.0, true, &ignore);
        assert_eq!(unsnapped, 244.0);
        assert_eq!(at_none, None);
    }

    #[test]
    fn resolve_overlaps_moves_colliding_clip_to_free_track() {
        let mut clips = vec![
            Clip {
                id: 1,
                kind: ClipKind::Key {
                    key: Some(KeyboardKey::KeyQ),
                },
                track: 0,
                start_ms: 100,
                dur_ms: 200,
            },
            Clip {
                id: 2,
                kind: ClipKind::Key {
                    key: Some(KeyboardKey::KeyE),
                },
                track: 0,
                start_ms: 150,
                dur_ms: 100,
            },
        ];
        let moved = HashSet::from([2]);
        resolve_overlaps(&mut clips, &moved, 1);
        assert_eq!(clips[1].track, 1);
    }

    #[test]
    fn razor_split_and_insert_gap_preserve_timeline_invariants() {
        let mut clips = vec![
            Clip {
                id: 1,
                kind: ClipKind::Mouse {
                    button: MouseButton::Left,
                },
                track: 0,
                start_ms: 0,
                dur_ms: 200,
            },
            Clip {
                id: 2,
                kind: ClipKind::Key {
                    key: Some(KeyboardKey::KeyW),
                },
                track: 1,
                start_ms: 250,
                dur_ms: 100,
            },
        ];
        let mut loop_region = Some(LoopRegion {
            start_ms: 100,
            end_ms: 400,
        });
        insert_time_gap(&mut clips, &mut loop_region, 100, 50);
        assert_eq!(clips[0].dur_ms, 250);
        assert_eq!(clips[1].start_ms, 300);
        assert_eq!(
            loop_region,
            Some(LoopRegion {
                start_ms: 150,
                end_ms: 450
            })
        );

        let mut next_id = 3;
        let created = razor_split_at(&mut clips, &HashSet::new(), 120, &mut next_id);
        assert_eq!(created, vec![3]);
        assert_eq!(clips[0].dur_ms, 120);
        assert_eq!(clips[2].start_ms, 120);
        assert_eq!(clips[2].dur_ms, 130);
    }

    #[test]
    fn compile_clips_respects_up_loop_end_loop_start_down_priority_and_round_trips() {
        let clips = vec![
            Clip {
                id: 1,
                kind: ClipKind::Key {
                    key: Some(KeyboardKey::KeyE),
                },
                track: 0,
                start_ms: 100,
                dur_ms: 200,
            },
            Clip {
                id: 2,
                kind: ClipKind::Key {
                    key: Some(KeyboardKey::KeyE),
                },
                track: 0,
                start_ms: 300,
                dur_ms: 100,
            },
            Clip {
                id: 3,
                kind: ClipKind::Mouse {
                    button: MouseButton::Left,
                },
                track: 1,
                start_ms: 150,
                dur_ms: 250,
            },
        ];
        let lp = Some(LoopRegion {
            start_ms: 100,
            end_ms: 400,
        });

        let steps = compile_clips(&clips, lp);
        assert_eq!(
            steps,
            vec![
                MacroStep::Wait(100),
                MacroStep::LoopStart,
                MacroStep::KeyDown(KeyboardKey::KeyE),
                MacroStep::Wait(50),
                MacroStep::MouseDown(MouseButton::Left),
                MacroStep::Wait(150),
                MacroStep::KeyUp(KeyboardKey::KeyE),
                MacroStep::KeyDown(KeyboardKey::KeyE),
                MacroStep::Wait(100),
                MacroStep::KeyUp(KeyboardKey::KeyE),
                MacroStep::MouseUp(MouseButton::Left),
                MacroStep::LoopEnd,
            ]
        );

        let mut next_id = 10;
        let (rebuilt_clips, rebuilt_loop, tracks) = steps_to_clips(&steps, &mut next_id);
        assert_eq!(rebuilt_loop, lp);
        assert_eq!(rebuilt_clips.len(), 3);
        assert!(tracks >= MIN_TRACKS);
    }

    #[test]
    fn validate_macro_timeline_catches_all_safety_and_structural_errors() {
        let ok_clip = Clip {
            id: 1,
            kind: ClipKind::Key {
                key: Some(KeyboardKey::KeyE),
            },
            track: 0,
            start_ms: 0,
            dur_ms: 100,
        };
        let stop = EmergencyStopHotkey::default();

        // Empty name
        assert!(
            validate_macro_timeline("   ", None, std::slice::from_ref(&ok_clip), None, stop)
                .is_err()
        );

        // Left/Right mouse hotkey blocked
        assert!(validate_macro_timeline(
            "Test",
            Some(InputSource::MouseButton(MouseButton::Left)),
            std::slice::from_ref(&ok_clip),
            None,
            stop
        )
        .is_err());

        // Self-trigger hotkey blocked
        assert!(validate_macro_timeline(
            "SelfLoop",
            Some(InputSource::Keyboard(KeyboardKey::KeyE)),
            std::slice::from_ref(&ok_clip),
            None,
            stop
        )
        .is_err());

        // Overlapping same key on 2 tracks blocked
        let clash_clip = Clip {
            id: 2,
            kind: ClipKind::Key {
                key: Some(KeyboardKey::KeyE),
            },
            track: 1,
            start_ms: 50,
            dur_ms: 100,
        };
        assert!(
            validate_macro_timeline("Clash", None, &[ok_clip.clone(), clash_clip], None, stop)
                .is_err()
        );

        // Loop straddling blocked
        assert!(validate_macro_timeline(
            "Straddle",
            None,
            std::slice::from_ref(&ok_clip),
            Some(LoopRegion {
                start_ms: 50,
                end_ms: 200
            }),
            stop
        )
        .is_err());

        // Valid macro passes
        assert!(validate_macro_timeline(
            "Combo E",
            Some(InputSource::MouseButton(MouseButton::XButton1)),
            &[ok_clip],
            Some(LoopRegion {
                start_ms: 0,
                end_ms: 150
            }),
            stop
        )
        .is_ok());
    }
}
