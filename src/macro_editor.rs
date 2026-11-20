//! Stateful NLE Multi-Track Macro Editor controller for the UI coordinator.
//!
//! Translates Slint pointer, keyboard, toolbar, inspector, and `import.md` modal events
//! into deterministic updates on pure [`crate::domain::macros`] models and generates
//! viewport-space geometry projections for `ui/components/macros_view.slint`.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::domain::config::EmergencyStopHotkey;
use crate::domain::keys::{InputSource, KeyboardKey, MouseButton};
use crate::domain::macro_io::{export_clips_to_macro_text, parse_macro_sequence};
use crate::domain::macros::{
    clashing_ids, compile_clips, format_compiled_preview, format_ruler_ms, free_track_at,
    insert_time_gap, keyboard_key_short_label, mouse_button_short_label, razor_split_at,
    resolve_overlaps, ruler_step, snap_playhead, snap_shift, snap_time, validate_macro_timeline,
    Clip, ClipKind, LoopRegion, MacroDefinition, MacroId, MacroPlaybackMode, MacroTrigger,
    TimelineSnapshot, DEFAULT_CLIP_MS, DEFAULT_ZOOM, DRAG_THRESHOLD, EDGE_PX, HEAD_W,
    HISTORY_LIMIT, MAX_MS, MIN_TRACKS, RULER_H, TRACK_H, ZOOM_MAX, ZOOM_MIN,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopEdge {
    Start,
    End,
}

#[derive(Debug, Clone)]
enum PointerDragMode {
    Scrub,
    LoopResize {
        edge: LoopEdge,
        before: TimelineSnapshot,
        moved: bool,
    },
    MoveClips {
        anchor_id: u32,
        x0: f32,
        y0: f32,
        ms0: f64,
        track0: isize,
        before: TimelineSnapshot,
        orig: HashMap<u32, (u64, u64, usize)>,
        moved: bool,
    },
    TrimStart {
        clip_id: u32,
        x0: f32,
        ms0: f64,
        before: TimelineSnapshot,
        orig_start: u64,
        orig_dur: u64,
        moved: bool,
    },
    TrimEnd {
        clip_id: u32,
        x0: f32,
        ms0: f64,
        before: TimelineSnapshot,
        orig_start: u64,
        orig_dur: u64,
        moved: bool,
    },
    Marquee {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        additive: bool,
        base: HashSet<u32>,
        moved: bool,
        click_ms: f64,
    },
}

#[derive(Debug, Clone)]
struct KeyCaptureState {
    clip_id: u32,
    chord_mode: bool,
    chord_keys: Vec<KeyboardKey>,
    snapshot_pushed: bool,
    created_empty_clip: bool,
}

#[derive(Debug, Clone)]
struct LiveRecordState {
    origin_ms: u64,
    t0: Option<Instant>,
    open_clips: HashMap<InputSource, u32>,
    last_now_ms: u64,
}

#[derive(Debug, Clone)]
struct PreviewPlaybackState {
    from_ms: u64,
    t0: Instant,
    now_ms: u64,
}

/// View projection of a timeline clip in viewport pixel coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct UiClipView {
    pub id: i32,
    pub label: String,
    pub dur_text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub kind_tag: i32,
    pub selected: bool,
    pub clashing: bool,
    pub live: bool,
}

/// View projection of a sticky time ruler tick mark.
#[derive(Debug, Clone, PartialEq)]
pub struct UiRulerTickView {
    pub x: f32,
    pub is_major: bool,
    pub label: String,
}

/// View projection of a sticky left track header (`T1..Tn`).
#[derive(Debug, Clone, PartialEq)]
pub struct UiTrackHeaderView {
    pub index: i32,
    pub label: String,
    pub y: f32,
    pub active: bool,
    pub can_delete: bool,
}

/// View projection for the Clip Inspector card.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiInspectorView {
    pub visible: bool,
    pub multi_mode: bool,
    pub title: String,
    pub is_key_clip: bool,
    pub key_label: String,
    pub mouse_button_idx: i32,
    pub start_ms_text: String,
    pub dur_ms_text: String,
    pub end_ms_text: String,
    pub track_text: String,
    pub group_start_text: String,
    pub group_dur_text: String,
}

/// Live validation badge summary for the `import.md` Import / Export modal.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiIoValidationView {
    pub valid: bool,
    pub message: String,
    pub clip_count: i32,
    pub track_count: i32,
    pub total_ms: i32,
    pub warning_text: String,
}

/// Complete state of the Multi-Track NLE Macro Studio editor.
#[derive(Debug)]
pub struct MacroEditorState {
    pub current_macro_id: Option<MacroId>,
    pub macro_name: String,
    pub hotkey: Option<InputSource>,
    pub playback_mode: MacroPlaybackMode,
    pub clips: Vec<Clip>,
    next_clip_id: u32,
    pub tracks: usize,
    pub loop_region: Option<LoopRegion>,
    pub playhead_ms: u64,
    pub zoom: f32,
    pub snap_enabled: bool,
    pub active_track: usize,
    pub selected_ids: HashSet<u32>,
    clipboard: Vec<Clip>,
    undo_stack: Vec<TimelineSnapshot>,
    redo_stack: Vec<TimelineSnapshot>,
    pub gap_input_ms: u64,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pointer_drag: Option<PointerDragMode>,
    pub hover_resize: bool,
    pub snap_guide_ms: Option<u64>,
    pub marquee_rect: Option<(f32, f32, f32, f32)>,
    key_capture: Option<KeyCaptureState>,
    pub capturing_hotkey: bool,
    recording: Option<LiveRecordState>,
    playing_preview: Option<PreviewPlaybackState>,
    pub io_modal_open: bool,
    pub editor_open: bool,
    pub io_text: String,
    pub io_replace_mode: bool,
    pub io_default_hold_ms: u64,
    pub io_validation: UiIoValidationView,
    pub help_open: bool,
    pub preview_open: bool,
    pub status_message: String,
    pub status_is_error: bool,
    pub status_is_success: bool,
}

impl Default for MacroEditorState {
    fn default() -> Self {
        let mut state = Self {
            current_macro_id: None,
            macro_name: String::new(),
            hotkey: None,
            playback_mode: MacroPlaybackMode::HoldLoop,
            clips: Vec::new(),
            next_clip_id: 1,
            tracks: MIN_TRACKS,
            loop_region: None,
            playhead_ms: 0,
            zoom: DEFAULT_ZOOM,
            snap_enabled: true,
            active_track: 0,
            selected_ids: HashSet::new(),
            clipboard: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            gap_input_ms: DEFAULT_CLIP_MS,
            scroll_x: 0.0,
            scroll_y: 0.0,
            viewport_w: 840.0,
            viewport_h: 340.0,
            pointer_drag: None,
            hover_resize: false,
            snap_guide_ms: None,
            marquee_rect: None,
            key_capture: None,
            capturing_hotkey: false,
            recording: None,
            playing_preview: None,
            io_modal_open: false,
            editor_open: false,
            io_text: String::new(),
            io_replace_mode: true,
            io_default_hold_ms: DEFAULT_CLIP_MS,
            io_validation: UiIoValidationView::default(),
            help_open: false,
            preview_open: false,
            status_message: "Sẵn sàng tạo macro mới".to_string(),
            status_is_error: false,
            status_is_success: false,
        };
        state.refresh_io_validation();
        state
    }
}

impl MacroEditorState {
    /// Returns the total effective track count (`>= MIN_TRACKS` and accommodating all clips).
    pub fn effective_tracks(&self) -> usize {
        let max_clip_track = self.clips.iter().map(|c| c.track + 1).max().unwrap_or(0);
        self.tracks.max(max_clip_track).max(MIN_TRACKS)
    }

    /// Returns the end timestamp in milliseconds of the last clip or loop region.
    pub fn content_end_ms(&self) -> u64 {
        let max_clip = self.clips.iter().map(Clip::end_ms).max().unwrap_or(0);
        let max_loop = self.loop_region.map_or(0, |lp| lp.end_ms);
        max_clip.max(max_loop)
    }

    /// Returns `true` if live recording is currently active.
    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    /// Returns `true` if preview playback is currently active.
    pub fn is_playing(&self) -> bool {
        self.playing_preview.is_some()
    }

    /// Returns `true` if the editor is actively capturing a clip key.
    pub fn is_capturing_clip_key(&self) -> bool {
        self.key_capture.is_some()
    }

    /// Returns a clear status prompt when capturing a clip key or hotkey, or empty string.
    pub fn capture_banner_text(&self) -> String {
        if self.capturing_hotkey {
            return "Đang chờ Hotkey kích hoạt: nhấn phím bất kỳ hoặc chuột giữa / M3 / M4 (Esc để huỷ · chặn chuột trái/phải)".to_string();
        }
        if let Some(cap) = &self.key_capture {
            if cap.chord_mode {
                return "Đang chờ bắt phím: bấm 1 phím hoặc giữ nhiều phím cùng lúc rồi thả để chèn song song (Esc để huỷ)".to_string();
            }
            return "Đang chờ đổi phím cho clip: bấm phím mới bất kỳ (Esc để huỷ)".to_string();
        }
        if self.recording.is_some() {
            return "ĐANG GHI TRỰC TIẾP: thao tác phím / chuột trên vùng timeline — bấm nút Ghi (■) lần nữa hoặc Emergency Stop để dừng".to_string();
        }
        String::new()
    }

    /// Updates viewport dimensions reported by Slint layout.
    pub fn set_viewport_size(&mut self, width_px: f32, height_px: f32) {
        if width_px > 100.0 {
            self.viewport_w = width_px;
        }
        if height_px > 100.0 {
            self.viewport_h = height_px;
        }
        self.clamp_scroll();
    }

    /// Clears the timeline and opens the Create Macro NLE Studio for a new macro draft.
    pub fn reset_new_macro(&mut self) {
        self.stop_all_active_modes();
        self.editor_open = true;
        self.current_macro_id = None;
        self.macro_name.clear();
        self.hotkey = None;
        self.playback_mode = MacroPlaybackMode::HoldLoop;
        self.clips.clear();
        self.tracks = MIN_TRACKS;
        self.loop_region = None;
        self.playhead_ms = 0;
        self.active_track = 0;
        self.selected_ids.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.set_info("Đã tạo bản nháp Macro mới");
    }

    /// Loads an existing [`MacroDefinition`] into the timeline editor and opens the studio view.
    pub fn load_macro(&mut self, mac: &MacroDefinition) {
        self.stop_all_active_modes();
        self.editor_open = true;
        self.current_macro_id = Some(mac.id);
        self.macro_name = mac.name.clone();
        self.hotkey = mac.trigger.map(|t| t.source);
        self.playback_mode = mac
            .trigger
            .map(|t| t.mode)
            .unwrap_or(MacroPlaybackMode::HoldLoop);
        self.clips = mac.clips.clone();
        self.next_clip_id = self.clips.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        self.tracks = mac.tracks.max(MIN_TRACKS);
        self.loop_region = mac.loop_region;
        self.playhead_ms = 0;
        self.active_track = 0;
        self.selected_ids.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.fit_to_viewport();
        self.set_info(format!("Đã nạp macro '{}'", mac.name));
    }

    /// Closes the Create Macro Studio and returns to the Macro List view.
    pub fn close_editor(&mut self) {
        self.stop_all_active_modes();
        self.io_modal_open = false;
        self.editor_open = false;
    }

    /// Immediately stops preview playback, live recording, and key/hotkey capture (e.g. on Emergency Stop).
    pub fn stop_all_active_modes(&mut self) {
        if self.recording.is_some() {
            self.stop_recording();
        }
        self.playing_preview = None;
        self.cancel_key_capture();
        self.capturing_hotkey = false;
        self.pointer_drag = None;
        self.snap_guide_ms = None;
        self.marquee_rect = None;
    }

    fn snapshot(&self) -> TimelineSnapshot {
        TimelineSnapshot {
            clips: self.clips.clone(),
            tracks: self.effective_tracks(),
            loop_region: self.loop_region,
        }
    }

    fn push_undo_snapshot(&mut self, snap: TimelineSnapshot) {
        if self.undo_stack.len() >= HISTORY_LIMIT {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(snap);
        self.redo_stack.clear();
    }

    fn push_current_undo(&mut self) {
        let snap = self.snapshot();
        self.push_undo_snapshot(snap);
    }

    fn restore_snapshot(&mut self, snap: TimelineSnapshot) {
        self.clips = snap.clips;
        self.tracks = snap.tracks.max(MIN_TRACKS);
        self.loop_region = snap.loop_region;
        self.next_clip_id = self
            .next_clip_id
            .max(self.clips.iter().map(|c| c.id).max().unwrap_or(0) + 1);
        self.selected_ids
            .retain(|id| self.clips.iter().any(|c| c.id == *id));
        self.active_track = self.active_track.min(self.effective_tracks() - 1);
        self.clamp_scroll();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty() && !self.is_recording()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty() && !self.is_recording()
    }

    pub fn can_razor(&self) -> bool {
        if self.is_recording() {
            return false;
        }
        let filter_selected = !self.selected_ids.is_empty();
        self.clips.iter().any(|c| {
            (!filter_selected || self.selected_ids.contains(&c.id))
                && c.start_ms < self.playhead_ms
                && c.end_ms() > self.playhead_ms
        })
    }

    pub fn undo(&mut self) {
        if self.is_recording() {
            return;
        }
        let Some(prev) = self.undo_stack.pop() else {
            return;
        };
        let current = self.snapshot();
        if self.redo_stack.len() >= HISTORY_LIMIT {
            self.redo_stack.remove(0);
        }
        self.redo_stack.push(current);
        self.restore_snapshot(prev);
        self.set_info("Đã hoàn tác");
    }

    pub fn redo(&mut self) {
        if self.is_recording() {
            return;
        }
        let Some(next) = self.redo_stack.pop() else {
            return;
        };
        let current = self.snapshot();
        if self.undo_stack.len() >= HISTORY_LIMIT {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(current);
        self.restore_snapshot(next);
        self.set_info("Đã làm lại");
    }

    pub fn toggle_snap(&mut self) {
        if self.is_recording() {
            return;
        }
        self.snap_enabled = !self.snap_enabled;
        self.set_info(if self.snap_enabled {
            "Đã bật hít từ tính (Snap)"
        } else {
            "Đã tắt hít từ tính"
        });
    }

    pub fn set_zoom(&mut self, next_zoom: f32, anchor_view_x: Option<f32>) {
        let next = next_zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        if (next - self.zoom).abs() < f32::EPSILON {
            return;
        }
        let view_x = match anchor_view_x {
            Some(vx) => vx.clamp(HEAD_W, self.viewport_w),
            None => (HEAD_W + (self.playhead_ms as f32) * self.zoom - self.scroll_x)
                .clamp(HEAD_W, self.viewport_w),
        };
        let anchor_ms = ((view_x + self.scroll_x - HEAD_W) / self.zoom).max(0.0);
        self.zoom = next;
        self.scroll_x = (HEAD_W + anchor_ms * next - view_x).max(0.0);
        self.clamp_scroll();
    }

    pub fn fit_to_viewport(&mut self) {
        let span = (self.content_end_ms().max(500)) as f32;
        let avail_w = (self.viewport_w - HEAD_W - 40.0).max(160.0);
        self.zoom = (avail_w / span).clamp(ZOOM_MIN, ZOOM_MAX);
        self.scroll_x = 0.0;
        self.clamp_scroll();
    }

    pub fn add_track(&mut self) {
        if self.is_recording() {
            return;
        }
        self.push_current_undo();
        self.tracks = self.effective_tracks() + 1;
        self.active_track = self.tracks - 1;
        let max_y = self.max_scroll_y();
        self.scroll_y = max_y;
        self.set_info(format!("Đã thêm track T{}", self.tracks));
    }

    pub fn remove_empty_track(&mut self, track_idx: usize) {
        if self.is_recording() || track_idx < MIN_TRACKS {
            return;
        }
        if self.clips.iter().any(|c| c.track == track_idx) {
            return;
        }
        self.push_current_undo();
        for clip in &mut self.clips {
            if clip.track > track_idx {
                clip.track -= 1;
            }
        }
        self.tracks = self.effective_tracks().saturating_sub(1).max(MIN_TRACKS);
        self.active_track = self.active_track.min(self.tracks - 1);
        self.clamp_scroll();
        self.set_info(format!("Đã xoá track trống T{}", track_idx + 1));
    }

    /// Inserts a mouse clip of `DEFAULT_CLIP_MS` at `(playhead_ms, active_track)` on the first free track.
    pub fn insert_mouse_clip(&mut self, button: MouseButton) {
        if self.is_recording() {
            return;
        }
        self.cancel_key_capture();
        self.push_current_undo();
        let start = self.playhead_ms.min(MAX_MS.saturating_sub(1));
        let dur = DEFAULT_CLIP_MS.min(MAX_MS.saturating_sub(start)).max(1);
        let track = free_track_at(&self.clips, start, dur, self.active_track, &HashSet::new());
        let id = self.alloc_clip_id();
        self.clips.push(Clip {
            id,
            kind: ClipKind::Mouse { button },
            track,
            start_ms: start,
            dur_ms: dur,
        });
        self.tracks = self.tracks.max(track + 1);
        self.selected_ids = HashSet::from([id]);
        self.active_track = track;
        self.set_info(format!(
            "Đã chèn {} tại {} ms (T{})",
            mouse_button_short_label(button),
            start,
            track + 1
        ));
    }

    /// Inserts an unassigned `?` keyboard clip at `(playhead_ms, active_track)` and starts chord key capture.
    pub fn insert_key_clip_and_capture(&mut self) {
        if self.is_recording() {
            return;
        }
        self.cancel_key_capture();
        self.push_current_undo();
        let start = self.playhead_ms.min(MAX_MS.saturating_sub(1));
        let dur = DEFAULT_CLIP_MS.min(MAX_MS.saturating_sub(start)).max(1);
        let track = free_track_at(&self.clips, start, dur, self.active_track, &HashSet::new());
        let id = self.alloc_clip_id();
        self.clips.push(Clip {
            id,
            kind: ClipKind::Key { key: None },
            track,
            start_ms: start,
            dur_ms: dur,
        });
        self.tracks = self.tracks.max(track + 1);
        self.selected_ids = HashSet::from([id]);
        self.active_track = track;
        self.key_capture = Some(KeyCaptureState {
            clip_id: id,
            chord_mode: true,
            chord_keys: Vec::new(),
            snapshot_pushed: true,
            created_empty_clip: true,
        });
        self.set_info("Nhấn phím (hoặc giữ nhiều phím cùng lúc rồi thả) để gán vào clip mới");
    }

    /// Inserts a time gap of `gap_input_ms` at the current playhead.
    pub fn insert_gap_at_playhead(&mut self) {
        if self.is_recording() || self.gap_input_ms == 0 {
            return;
        }
        self.push_current_undo();
        insert_time_gap(
            &mut self.clips,
            &mut self.loop_region,
            self.playhead_ms,
            self.gap_input_ms,
        );
        self.set_info(format!(
            "Đã chèn khoảng trống {} ms tại {} ms",
            self.gap_input_ms, self.playhead_ms
        ));
    }

    pub fn cut_selected(&mut self) {
        if self.is_recording() || self.selected_ids.is_empty() {
            return;
        }
        self.copy_selected();
        self.delete_selected();
    }

    pub fn copy_selected(&mut self) {
        if self.selected_ids.is_empty() {
            return;
        }
        let chosen: Vec<&Clip> = self
            .clips
            .iter()
            .filter(|c| self.selected_ids.contains(&c.id))
            .collect();
        if chosen.is_empty() {
            return;
        }
        let min_start = chosen.iter().map(|c| c.start_ms).min().unwrap_or(0);
        let min_track = chosen.iter().map(|c| c.track).min().unwrap_or(0);
        self.clipboard = chosen
            .into_iter()
            .map(|c| Clip {
                id: 0,
                kind: c.kind,
                track: c.track - min_track,
                start_ms: c.start_ms - min_start,
                dur_ms: c.dur_ms,
            })
            .collect();
        self.set_info(format!("Đã copy {} clip", self.clipboard.len()));
    }

    pub fn paste_at_playhead(&mut self) {
        if self.is_recording() || self.clipboard.is_empty() {
            return;
        }
        self.push_current_undo();
        let placed =
            self.place_relative_clips(&self.clipboard.clone(), self.playhead_ms, self.active_track);
        self.selected_ids = placed.into_iter().collect();
        self.set_info(format!("Đã dán tại {} ms", self.playhead_ms));
    }

    pub fn duplicate_selected(&mut self) {
        if self.is_recording() || self.selected_ids.is_empty() {
            return;
        }
        let chosen: Vec<&Clip> = self
            .clips
            .iter()
            .filter(|c| self.selected_ids.contains(&c.id))
            .collect();
        if chosen.is_empty() {
            return;
        }
        let min_start = chosen.iter().map(|c| c.start_ms).min().unwrap_or(0);
        let max_end = chosen.iter().map(|c| c.end_ms()).max().unwrap_or(0);
        let min_track = chosen.iter().map(|c| c.track).min().unwrap_or(0);
        let rel: Vec<Clip> = chosen
            .into_iter()
            .map(|c| Clip {
                id: 0,
                kind: c.kind,
                track: c.track - min_track,
                start_ms: c.start_ms - min_start,
                dur_ms: c.dur_ms,
            })
            .collect();
        self.push_current_undo();
        let placed = self.place_relative_clips(&rel, max_end, min_track);
        self.selected_ids = placed.into_iter().collect();
        self.set_info("Đã nhân đôi clip ngay phía sau");
    }

    fn place_relative_clips(
        &mut self,
        rel: &[Clip],
        origin_ms: u64,
        origin_track: usize,
    ) -> Vec<u32> {
        let mut added_ids = Vec::with_capacity(rel.len());
        let mut added_set = HashSet::new();
        for template in rel {
            let id = self.alloc_clip_id();
            let start_ms = origin_ms.saturating_add(template.start_ms).min(MAX_MS - 1);
            let dur_ms = template.dur_ms.min(MAX_MS.saturating_sub(start_ms)).max(1);
            self.clips.push(Clip {
                id,
                kind: template.kind,
                track: origin_track + template.track,
                start_ms,
                dur_ms,
            });
            added_ids.push(id);
            added_set.insert(id);
        }
        resolve_overlaps(&mut self.clips, &added_set, 1);
        let max_t = self.clips.iter().map(|c| c.track + 1).max().unwrap_or(0);
        self.tracks = self.tracks.max(max_t);
        added_ids
    }

    pub fn razor_at_playhead(&mut self) {
        if !self.can_razor() {
            return;
        }
        self.push_current_undo();
        let created = razor_split_at(
            &mut self.clips,
            &self.selected_ids,
            self.playhead_ms,
            &mut self.next_clip_id,
        );
        if !created.is_empty() {
            let count = created.len();
            self.selected_ids = created.into_iter().collect();
            self.set_info(format!(
                "Đã cắt đôi {count} clip tại {} ms",
                self.playhead_ms
            ));
        }
    }

    pub fn delete_selected(&mut self) {
        if self.is_recording() || self.selected_ids.is_empty() {
            return;
        }
        self.push_current_undo();
        let del = self.selected_ids.clone();
        self.clips.retain(|c| !del.contains(&c.id));
        self.selected_ids.clear();
        self.set_info("Đã xoá clip đã chọn");
    }

    pub fn clear_timeline(&mut self) {
        if self.is_recording() || (self.clips.is_empty() && self.loop_region.is_none()) {
            return;
        }
        self.push_current_undo();
        self.clips.clear();
        self.loop_region = None;
        self.selected_ids.clear();
        self.set_info("Đã xoá sạch timeline");
    }

    pub fn set_loop_in(&mut self) {
        if self.is_recording() {
            return;
        }
        self.push_current_undo();
        let end = match self.loop_region {
            Some(lp) if lp.end_ms > self.playhead_ms => lp.end_ms,
            _ => self
                .content_end_ms()
                .max(self.playhead_ms.saturating_add(100))
                .min(MAX_MS),
        };
        if end <= self.playhead_ms {
            self.set_error("Điểm đầu vùng lặp phải nhỏ hơn điểm cuối.");
            return;
        }
        self.loop_region = Some(LoopRegion {
            start_ms: self.playhead_ms,
            end_ms: end,
        });
        self.set_info(format!("Đặt đầu vùng lặp: {} ms", self.playhead_ms));
    }

    pub fn set_loop_out(&mut self) {
        if self.is_recording() {
            return;
        }
        let start = match self.loop_region {
            Some(lp) if lp.start_ms < self.playhead_ms => lp.start_ms,
            _ => 0,
        };
        if self.playhead_ms <= start {
            self.set_error("Điểm cuối vùng lặp phải lớn hơn điểm đầu (> 0 ms).");
            return;
        }
        self.push_current_undo();
        self.loop_region = Some(LoopRegion {
            start_ms: start,
            end_ms: self.playhead_ms,
        });
        self.set_info(format!("Đặt cuối vùng lặp: {} ms", self.playhead_ms));
    }

    pub fn clear_loop(&mut self) {
        if self.is_recording() || self.loop_region.is_none() {
            return;
        }
        self.push_current_undo();
        self.loop_region = None;
        self.set_info("Đã bỏ vùng lặp");
    }

    /// Toggles real-time preview playback (`Space`).
    pub fn toggle_playback(&mut self) {
        if self.is_recording() {
            return;
        }
        if self.playing_preview.is_some() {
            self.playing_preview = None;
            self.set_info("Đã dừng xem thử");
            return;
        }
        if self.clips.is_empty() {
            return;
        }
        self.cancel_key_capture();
        let content_end = self.content_end_ms();
        let from_ms = if self.playhead_ms >= content_end {
            0
        } else {
            self.playhead_ms
        };
        self.playhead_ms = from_ms;
        self.playing_preview = Some(PreviewPlaybackState {
            from_ms,
            t0: Instant::now(),
            now_ms: from_ms,
        });
        self.set_info("Đang chạy xem thử timeline...");
    }

    /// Toggles Live Recording mode.
    pub fn toggle_recording(&mut self) {
        if self.recording.is_some() {
            self.stop_recording();
            return;
        }
        self.playing_preview = None;
        self.cancel_key_capture();
        self.capturing_hotkey = false;
        self.push_current_undo();
        self.recording = Some(LiveRecordState {
            origin_ms: self.playhead_ms,
            t0: None,
            open_clips: HashMap::new(),
            last_now_ms: self.playhead_ms,
        });
        self.set_info("Đang chờ thao tác đầu tiên để bắt đầu ghi...");
    }

    fn stop_recording(&mut self) {
        let Some(rec) = self.recording.take() else {
            return;
        };
        let end_now = rec.last_now_ms;
        for (_, clip_id) in rec.open_clips {
            if let Some(clip) = self.clips.iter_mut().find(|c| c.id == clip_id) {
                clip.dur_ms = end_now.saturating_sub(clip.start_ms).max(1);
            }
        }
        self.set_info("Đã dừng ghi trực tiếp");
    }

    /// Advances preview playback or live recording durations on each 16ms UI timer tick.
    ///
    /// Returns `true` if the UI should repaint/re-sync.
    pub fn tick_animation(&mut self) -> bool {
        let content_end = self.content_end_ms();
        if let Some(pb) = &mut self.playing_preview {
            let elapsed_ms = pb.t0.elapsed().as_millis() as u64;
            let mut now = pb.from_ms.saturating_add(elapsed_ms);

            if let Some(lp) = self.loop_region {
                if lp.end_ms > lp.start_ms && now >= lp.end_ms && pb.from_ms < lp.end_ms {
                    let period = lp.end_ms - lp.start_ms;
                    now = lp.start_ms + ((now - lp.start_ms) % period);
                } else if now >= content_end {
                    self.playhead_ms = content_end;
                    self.playing_preview = None;
                    self.set_info("Xem thử hoàn tất");
                    return true;
                }
            } else if now >= content_end {
                self.playhead_ms = content_end;
                self.playing_preview = None;
                self.set_info("Xem thử hoàn tất");
                return true;
            }

            pb.now_ms = now;
            self.playhead_ms = now;
            self.scroll_time_into_view(now);
            return true;
        }

        let rec_update = if let Some(rec) = &mut self.recording {
            let Some(t0) = rec.t0 else {
                return false;
            };
            let now = rec
                .origin_ms
                .saturating_add(t0.elapsed().as_millis() as u64)
                .min(MAX_MS);
            rec.last_now_ms = now;
            let ids: Vec<u32> = rec.open_clips.values().copied().collect();
            Some((now, ids))
        } else {
            None
        };

        if let Some((now, open_ids)) = rec_update {
            self.playhead_ms = now;
            for clip_id in open_ids {
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == clip_id) {
                    clip.dur_ms = now.saturating_sub(clip.start_ms).max(1);
                }
            }
            self.scroll_time_into_view(now);
            return true;
        }

        false
    }

    /// Scrolls horizontally just enough to keep `ms` visible inside the NLE viewport.
    pub fn scroll_time_into_view(&mut self, ms: u64) {
        let local_x = HEAD_W + (ms as f32) * self.zoom;
        let view_right = self.scroll_x + self.viewport_w - 24.0;
        let view_left = self.scroll_x + HEAD_W + 12.0;
        if local_x > view_right {
            self.scroll_x = (local_x - (self.viewport_w - 36.0)).max(0.0);
        } else if local_x < view_left {
            self.scroll_x = (local_x - HEAD_W - 24.0).max(0.0);
        }
        self.clamp_scroll();
    }

    pub fn set_playhead_ms(&mut self, ms: u64) {
        if self.is_recording() {
            return;
        }
        self.playhead_ms = ms.min(MAX_MS);
        self.scroll_time_into_view(self.playhead_ms);
    }

    pub fn on_pointer_down(
        &mut self,
        vx: f32,
        vy: f32,
        button_code: i32,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) {
        if self.capturing_hotkey {
            let mapped = match button_code {
                0 => Some(MouseButton::Left),
                1 => Some(MouseButton::Right),
                2 => Some(MouseButton::Middle),
                3 => Some(MouseButton::XButton1),
                4 => Some(MouseButton::XButton2),
                _ => None,
            };
            if let Some(btn) = mapped {
                if matches!(btn, MouseButton::Left | MouseButton::Right) {
                    self.set_error(
                        "Không thể dùng Chuột trái/phải làm Hotkey. Hãy chọn phím hoặc MC/M3/M4.",
                    );
                } else {
                    self.hotkey = Some(InputSource::MouseButton(btn));
                    self.capturing_hotkey = false;
                    self.set_info(format!("Đã gán Hotkey: {}", btn.display_name()));
                }
            }
            return;
        }

        if let Some(btn) = mouse_button_from_code(button_code) {
            if self.recording.is_some() {
                self.record_input_down(InputSource::MouseButton(btn));
                return;
            }
        }

        if button_code != 0 {
            return;
        }

        if self.key_capture.is_some() {
            self.cancel_key_capture();
        }
        self.playing_preview = None;

        // 1. Track header column
        if vx <= HEAD_W && vy > RULER_H {
            let track_isize = self.view_y_to_track(vy);
            let max_tracks = self.effective_tracks();
            if track_isize >= 0 && (track_isize as usize) < max_tracks {
                let track = track_isize as usize;
                let can_delete =
                    track >= MIN_TRACKS && !self.clips.iter().any(|c| c.track == track);
                if can_delete && vx >= HEAD_W - 22.0 {
                    self.remove_empty_track(track);
                } else {
                    self.active_track = track;
                }
            }
            return;
        }

        // 2. Top ruler (Loop handles or Playhead scrub)
        if vy <= RULER_H && vx > HEAD_W {
            if let Some(lp) = self.loop_region {
                let x_start = self.ms_to_view_x(lp.start_ms);
                let x_end = self.ms_to_view_x(lp.end_ms);
                if (vx - x_start).abs() <= 8.0 {
                    self.pointer_drag = Some(PointerDragMode::LoopResize {
                        edge: LoopEdge::Start,
                        before: self.snapshot(),
                        moved: false,
                    });
                    return;
                }
                if (vx - x_end).abs() <= 8.0 {
                    self.pointer_drag = Some(PointerDragMode::LoopResize {
                        edge: LoopEdge::End,
                        before: self.snapshot(),
                        moved: false,
                    });
                    return;
                }
            }
            let click_ms = self.view_x_to_ms(vx);
            self.playhead_ms = snap_playhead(
                &self.clips,
                self.loop_region,
                self.zoom,
                click_ms,
                !self.snap_enabled || alt,
            );
            self.pointer_drag = Some(PointerDragMode::Scrub);
            return;
        }

        // 3. Track lanes: hit-test clips (selected clips prioritized, reverse paint order)
        if vy > RULER_H && vx > HEAD_W {
            if let Some(hit_idx) = self.hit_test_clip(vx, vy) {
                let clip_id = self.clips[hit_idx].id;
                let clip_track = self.clips[hit_idx].track;
                let clip_x = self.ms_to_view_x(self.clips[hit_idx].start_ms);
                let clip_w = ((self.clips[hit_idx].dur_ms as f32) * self.zoom).max(3.0);

                if ctrl {
                    if !self.selected_ids.remove(&clip_id) {
                        self.selected_ids.insert(clip_id);
                    }
                    return;
                }
                if shift {
                    self.selected_ids.insert(clip_id);
                } else if !self.selected_ids.contains(&clip_id) {
                    self.selected_ids.clear();
                    self.selected_ids.insert(clip_id);
                }
                self.active_track = clip_track;

                let before = self.snapshot();
                let ms0 = self.view_x_to_ms(vx);

                if clip_w > EDGE_PX * 3.0 {
                    if vx - clip_x <= EDGE_PX {
                        self.pointer_drag = Some(PointerDragMode::TrimStart {
                            clip_id,
                            x0: vx,
                            ms0,
                            before,
                            orig_start: self.clips[hit_idx].start_ms,
                            orig_dur: self.clips[hit_idx].dur_ms,
                            moved: false,
                        });
                        return;
                    }
                    if (clip_x + clip_w) - vx <= EDGE_PX {
                        self.pointer_drag = Some(PointerDragMode::TrimEnd {
                            clip_id,
                            x0: vx,
                            ms0,
                            before,
                            orig_start: self.clips[hit_idx].start_ms,
                            orig_dur: self.clips[hit_idx].dur_ms,
                            moved: false,
                        });
                        return;
                    }
                }

                let mut orig = HashMap::new();
                for c in &self.clips {
                    if self.selected_ids.contains(&c.id) {
                        orig.insert(c.id, (c.start_ms, c.dur_ms, c.track));
                    }
                }
                self.pointer_drag = Some(PointerDragMode::MoveClips {
                    anchor_id: clip_id,
                    x0: vx,
                    y0: vy,
                    ms0,
                    track0: self.view_y_to_track(vy),
                    before,
                    orig,
                    moved: false,
                });
                return;
            }

            // 4. Empty track lane -> start Marquee selection / click-scrub
            let track = self
                .view_y_to_track(vy)
                .clamp(0, self.effective_tracks() as isize - 1) as usize;
            self.active_track = track;
            let click_ms = self.view_x_to_ms(vx);
            self.pointer_drag = Some(PointerDragMode::Marquee {
                x0: vx,
                y0: vy,
                x1: vx,
                y1: vy,
                additive: ctrl || shift,
                base: self.selected_ids.clone(),
                moved: false,
                click_ms,
            });
        }
    }

    pub fn on_pointer_move(&mut self, vx: f32, vy: f32, alt: bool) {
        let snap_disabled = !self.snap_enabled || alt;
        let Some(mut drag) = self.pointer_drag.take() else {
            self.update_hover_cursor(vx, vy);
            return;
        };

        match &mut drag {
            PointerDragMode::Scrub => {
                let ms = self.view_x_to_ms(vx);
                self.playhead_ms =
                    snap_playhead(&self.clips, self.loop_region, self.zoom, ms, snap_disabled);
            }
            PointerDragMode::LoopResize { edge, moved, .. } => {
                *moved = true;
                let raw_ms = self.view_x_to_ms(vx).clamp(0.0, MAX_MS as f64);
                let (snapped, snap_at) = snap_time(
                    &self.clips,
                    self.playhead_ms,
                    None,
                    self.zoom,
                    raw_ms,
                    snap_disabled,
                    &HashSet::new(),
                );
                self.snap_guide_ms = snap_at;
                let val = snapped.round().clamp(0.0, MAX_MS as f64) as u64;
                if let Some(lp) = self.loop_region.as_mut() {
                    match edge {
                        LoopEdge::Start => lp.start_ms = val.min(lp.end_ms.saturating_sub(1)),
                        LoopEdge::End => lp.end_ms = val.max(lp.start_ms.saturating_add(1)),
                    }
                }
            }
            PointerDragMode::MoveClips {
                x0,
                y0,
                ms0,
                track0,
                orig,
                moved,
                ..
            } => {
                if !*moved && (*x0 - vx).hypot(*y0 - vy) < DRAG_THRESHOLD {
                    self.pointer_drag = Some(drag);
                    return;
                }
                *moved = true;
                let raw_shift_ms = self.view_x_to_ms(vx) - *ms0;
                let min_start = orig.values().map(|v| v.0).min().unwrap_or(0) as f64;
                let max_end = orig.values().map(|v| v.0 + v.1).max().unwrap_or(0) as f64;
                let clamped_raw = raw_shift_ms.clamp(-min_start, MAX_MS as f64 - max_end);

                let orig_pairs: Vec<(u64, u64)> = orig.values().map(|v| (v.0, v.1)).collect();
                let ignore: HashSet<u32> = orig.keys().copied().collect();
                let (snapped_shift, snap_at) = snap_shift(
                    &self.clips,
                    self.playhead_ms,
                    self.loop_region,
                    self.zoom,
                    &orig_pairs,
                    clamped_raw,
                    snap_disabled,
                    &ignore,
                );
                let shift_ms = snapped_shift
                    .clamp(-min_start, MAX_MS as f64 - max_end)
                    .round() as i64;
                self.snap_guide_ms = snap_at;

                let cur_track = self.view_y_to_track(vy);
                let min_track = orig.values().map(|v| v.2).min().unwrap_or(0) as isize;
                let shift_track = (cur_track - *track0).max(-min_track);

                for clip in &mut self.clips {
                    if let Some(&(os, _od, ot)) = orig.get(&clip.id) {
                        clip.start_ms = (os as i64 + shift_ms).max(0) as u64;
                        clip.track = (ot as isize + shift_track).max(0) as usize;
                    }
                }
            }
            PointerDragMode::TrimStart {
                clip_id,
                x0,
                ms0,
                orig_start,
                orig_dur,
                moved,
                ..
            } => {
                if !*moved && (vx - *x0).abs() < DRAG_THRESHOLD {
                    self.pointer_drag = Some(drag);
                    return;
                }
                *moved = true;
                let orig_end = orig_start.saturating_add(*orig_dur);
                let Some(track) = self
                    .clips
                    .iter()
                    .find(|c| c.id == *clip_id)
                    .map(|c| c.track)
                else {
                    return;
                };
                let limit_left = self
                    .clips
                    .iter()
                    .filter(|c| c.id != *clip_id && c.track == track && c.end_ms() <= *orig_start)
                    .map(Clip::end_ms)
                    .max()
                    .unwrap_or(0);

                let d_ms = self.view_x_to_ms(vx) - *ms0;
                let ignore = HashSet::from([*clip_id]);
                let (snapped, snap_at) = snap_time(
                    &self.clips,
                    self.playhead_ms,
                    self.loop_region,
                    self.zoom,
                    *orig_start as f64 + d_ms,
                    snap_disabled,
                    &ignore,
                );
                self.snap_guide_ms = snap_at;
                let new_start =
                    (snapped.round() as i64).clamp(limit_left as i64, (orig_end - 1) as i64) as u64;
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == *clip_id) {
                    clip.start_ms = new_start;
                    clip.dur_ms = orig_end - new_start;
                }
            }
            PointerDragMode::TrimEnd {
                clip_id,
                x0,
                ms0,
                orig_start,
                orig_dur,
                moved,
                ..
            } => {
                if !*moved && (vx - *x0).abs() < DRAG_THRESHOLD {
                    self.pointer_drag = Some(drag);
                    return;
                }
                *moved = true;
                let orig_end = orig_start.saturating_add(*orig_dur);
                let Some(track) = self
                    .clips
                    .iter()
                    .find(|c| c.id == *clip_id)
                    .map(|c| c.track)
                else {
                    return;
                };
                let limit_right = self
                    .clips
                    .iter()
                    .filter(|c| c.id != *clip_id && c.track == track && c.start_ms >= orig_end)
                    .map(|c| c.start_ms)
                    .min()
                    .unwrap_or(MAX_MS);

                let d_ms = self.view_x_to_ms(vx) - *ms0;
                let ignore = HashSet::from([*clip_id]);
                let (snapped, snap_at) = snap_time(
                    &self.clips,
                    self.playhead_ms,
                    self.loop_region,
                    self.zoom,
                    orig_end as f64 + d_ms,
                    snap_disabled,
                    &ignore,
                );
                self.snap_guide_ms = snap_at;
                let new_end = (snapped.round() as i64)
                    .clamp((*orig_start + 1) as i64, limit_right as i64)
                    as u64;
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == *clip_id) {
                    clip.dur_ms = new_end - *orig_start;
                }
            }
            PointerDragMode::Marquee {
                x0,
                y0,
                x1,
                y1,
                additive,
                base,
                moved,
                ..
            } => {
                *x1 = vx.max(HEAD_W);
                *y1 = vy.max(RULER_H);
                if !*moved && (*x1 - *x0).hypot(*y1 - *y0) >= DRAG_THRESHOLD {
                    *moved = true;
                }
                if *moved {
                    let rx0 = x0.min(*x1);
                    let ry0 = y0.min(*y1);
                    let rw = (*x1 - *x0).abs();
                    let rh = (*y1 - *y0).abs();
                    self.marquee_rect = Some((rx0, ry0, rw, rh));

                    let mut sel = if *additive {
                        base.clone()
                    } else {
                        HashSet::new()
                    };
                    for clip in &self.clips {
                        let cx = self.ms_to_view_x(clip.start_ms);
                        let cw = ((clip.dur_ms as f32) * self.zoom).max(3.0);
                        let cy = self.track_to_view_y(clip.track) + 3.0;
                        let ch = TRACK_H - 6.0;
                        if cx < rx0 + rw && cx + cw > rx0 && cy < ry0 + rh && cy + ch > ry0 {
                            sel.insert(clip.id);
                        }
                    }
                    self.selected_ids = sel;
                }
            }
        }

        self.pointer_drag = Some(drag);
    }

    pub fn on_pointer_up(&mut self, button_code: i32, ctrl: bool, shift: bool, alt: bool) {
        if let Some(btn) = mouse_button_from_code(button_code) {
            if self.recording.is_some() {
                self.record_input_up(InputSource::MouseButton(btn));
                return;
            }
        }

        let Some(drag) = self.pointer_drag.take() else {
            return;
        };
        self.snap_guide_ms = None;
        self.marquee_rect = None;

        match drag {
            PointerDragMode::Scrub => {}
            PointerDragMode::LoopResize { before, moved, .. } => {
                if moved {
                    self.push_undo_snapshot(before);
                }
            }
            PointerDragMode::MoveClips {
                anchor_id,
                before,
                orig,
                moved,
                ..
            } => {
                if !moved {
                    if !shift && self.selected_ids.len() > 1 {
                        self.selected_ids = HashSet::from([anchor_id]);
                    }
                } else {
                    let moved_ids: HashSet<u32> = orig.keys().copied().collect();
                    let orig_track = orig.get(&anchor_id).map(|v| v.2).unwrap_or(0) as isize;
                    let new_track = self
                        .clips
                        .iter()
                        .find(|c| c.id == anchor_id)
                        .map(|c| c.track as isize)
                        .unwrap_or(orig_track);
                    let dir = if new_track < orig_track { -1 } else { 1 };
                    resolve_overlaps(&mut self.clips, &moved_ids, dir);
                    let max_t = self.clips.iter().map(|c| c.track + 1).max().unwrap_or(0);
                    self.tracks = self.tracks.max(max_t);
                    self.push_undo_snapshot(before);
                }
            }
            PointerDragMode::TrimStart { before, moved, .. }
            | PointerDragMode::TrimEnd { before, moved, .. } => {
                if moved {
                    self.push_undo_snapshot(before);
                }
            }
            PointerDragMode::Marquee {
                moved, click_ms, ..
            } => {
                if !moved {
                    if !ctrl && !shift {
                        self.selected_ids.clear();
                    }
                    self.playhead_ms = snap_playhead(
                        &self.clips,
                        self.loop_region,
                        self.zoom,
                        click_ms,
                        !self.snap_enabled || alt,
                    );
                }
            }
        }
    }

    pub fn on_double_click(&mut self, vx: f32, vy: f32, alt: bool) {
        if self.is_recording() || vx <= HEAD_W || vy <= RULER_H {
            return;
        }
        if let Some(hit_idx) = self.hit_test_clip(vx, vy) {
            let clip_id = self.clips[hit_idx].id;
            if matches!(self.clips[hit_idx].kind, ClipKind::Key { .. }) {
                self.selected_ids = HashSet::from([clip_id]);
                self.key_capture = Some(KeyCaptureState {
                    clip_id,
                    chord_mode: false,
                    chord_keys: Vec::new(),
                    snapshot_pushed: false,
                    created_empty_clip: false,
                });
                self.set_info("Nhấn phím mới để thay thế cho clip đã chọn (Esc để huỷ)");
            }
            return;
        }

        // Double click empty lane -> create unassigned key clip at clicked position and capture chord
        let track = self
            .view_y_to_track(vy)
            .clamp(0, self.effective_tracks() as isize - 1) as usize;
        let click_ms = self.view_x_to_ms(vx);
        let at = snap_playhead(
            &self.clips,
            self.loop_region,
            self.zoom,
            click_ms,
            !self.snap_enabled || alt,
        )
        .min(MAX_MS - 1);
        self.push_current_undo();
        self.playhead_ms = at;
        let dur = DEFAULT_CLIP_MS.min(MAX_MS.saturating_sub(at)).max(1);
        let free_t = free_track_at(&self.clips, at, dur, track, &HashSet::new());
        let id = self.alloc_clip_id();
        self.clips.push(Clip {
            id,
            kind: ClipKind::Key { key: None },
            track: free_t,
            start_ms: at,
            dur_ms: dur,
        });
        self.tracks = self.tracks.max(free_t + 1);
        self.active_track = free_t;
        self.selected_ids = HashSet::from([id]);
        self.key_capture = Some(KeyCaptureState {
            clip_id: id,
            chord_mode: true,
            chord_keys: Vec::new(),
            snapshot_pushed: true,
            created_empty_clip: true,
        });
        self.set_info("Nhấn phím (hoặc giữ tổ hợp phím rồi thả) để gán tại vị trí vừa nhấp đúp");
    }

    pub fn on_wheel(&mut self, vx: f32, delta_x: f32, delta_y: f32, ctrl: bool, shift: bool) {
        if ctrl {
            let factor = if delta_y >= 0.0 { 1.15 } else { 1.0 / 1.15 };
            self.set_zoom(self.zoom * factor, Some(vx));
            return;
        }
        let max_y = self.max_scroll_y();
        if shift || max_y <= 0.5 {
            self.scroll_x = (self.scroll_x - delta_y - delta_x).clamp(0.0, self.max_scroll_x());
        } else {
            self.scroll_y = (self.scroll_y - delta_y).clamp(0.0, max_y);
            self.scroll_x = (self.scroll_x - delta_x).clamp(0.0, self.max_scroll_x());
        }
    }

    /// Handles a Slint key-pressed event inside the timeline canvas or capture modal.
    ///
    /// Returns `Some(true)` if the caller should trigger Save (`Ctrl+S`), or `Some(false)` if handled.
    pub fn on_key_pressed(
        &mut self,
        text: &str,
        repeat: bool,
        ctrl: bool,
        shift: bool,
        alt: bool,
    ) -> bool {
        let key_opt = slint_text_to_keyboard_key(text);

        if self.capturing_hotkey {
            if key_opt == Some(KeyboardKey::Escape) {
                self.capturing_hotkey = false;
                self.set_info("Đã huỷ chọn Hotkey");
                return false;
            }
            if let Some(key) = key_opt {
                self.hotkey = Some(InputSource::Keyboard(key));
                self.capturing_hotkey = false;
                self.set_info(format!("Đã gán Hotkey: {}", key.display_name()));
            }
            return false;
        }

        if let Some(mut cap) = self.key_capture.take() {
            if key_opt == Some(KeyboardKey::Escape) {
                if cap.created_empty_clip {
                    self.clips.retain(|c| c.id != cap.clip_id);
                    self.selected_ids.remove(&cap.clip_id);
                    self.undo_stack.pop();
                }
                self.set_info("Đã huỷ bắt phím");
                return false;
            }
            if repeat {
                self.key_capture = Some(cap);
                return false;
            }
            if let Some(key) = key_opt {
                if !cap.chord_mode {
                    if !cap.snapshot_pushed {
                        self.push_current_undo();
                    }
                    if let Some(clip) = self.clips.iter_mut().find(|c| c.id == cap.clip_id) {
                        clip.kind = ClipKind::Key { key: Some(key) };
                    }
                    self.set_info(format!("Đã đổi phím thành {}", key.display_name()));
                    return false;
                }
                if !cap.chord_keys.contains(&key) {
                    cap.chord_keys.push(key);
                }
                if let Some(clip) = self.clips.iter_mut().find(|c| c.id == cap.clip_id) {
                    clip.kind = ClipKind::Key {
                        key: Some(cap.chord_keys[0]),
                    };
                }
                self.key_capture = Some(cap);
            } else {
                self.key_capture = Some(cap);
            }
            return false;
        }

        if self.recording.is_some() {
            if !repeat {
                if let Some(key) = key_opt {
                    self.record_input_down(InputSource::Keyboard(key));
                }
            }
            return false;
        }

        // Global Timeline Shortcuts
        if ctrl {
            match key_opt {
                Some(KeyboardKey::KeyZ) => {
                    self.undo();
                    return false;
                }
                Some(KeyboardKey::KeyY) => {
                    self.redo();
                    return false;
                }
                Some(KeyboardKey::KeyS) => return true,
                Some(KeyboardKey::KeyX) => {
                    self.cut_selected();
                    return false;
                }
                Some(KeyboardKey::KeyC) => {
                    self.copy_selected();
                    return false;
                }
                Some(KeyboardKey::KeyV) => {
                    self.paste_at_playhead();
                    return false;
                }
                Some(KeyboardKey::KeyD) => {
                    self.duplicate_selected();
                    return false;
                }
                Some(KeyboardKey::KeyA) => {
                    self.selected_ids = self.clips.iter().map(|c| c.id).collect();
                    return false;
                }
                _ => {}
            }
        }

        match key_opt {
            Some(KeyboardKey::Delete | KeyboardKey::Backspace) => self.delete_selected(),
            Some(KeyboardKey::Space) => self.toggle_playback(),
            Some(KeyboardKey::KeyC) if !ctrl => self.razor_at_playhead(),
            Some(KeyboardKey::KeyI) if !ctrl => self.set_loop_in(),
            Some(KeyboardKey::KeyO) if !ctrl => self.set_loop_out(),
            Some(KeyboardKey::Home) => {
                self.set_playhead_ms(0);
            }
            Some(KeyboardKey::End) => {
                let end = self.content_end_ms();
                self.set_playhead_ms(end);
            }
            Some(KeyboardKey::Equal) => self.set_zoom(self.zoom * 1.25, None),
            Some(KeyboardKey::Minus) => self.set_zoom(self.zoom * 0.8, None),
            Some(KeyboardKey::ArrowLeft) => self.nudge_horizontal(if shift { -10 } else { -1 }),
            Some(KeyboardKey::ArrowRight) => self.nudge_horizontal(if shift { 10 } else { 1 }),
            Some(KeyboardKey::ArrowUp) => self.nudge_vertical(-1),
            Some(KeyboardKey::ArrowDown) => self.nudge_vertical(1),
            Some(KeyboardKey::Escape) => self.selected_ids.clear(),
            _ => {
                let _ = alt;
            }
        }
        false
    }

    pub fn on_key_released(&mut self, text: &str) {
        let key_opt = slint_text_to_keyboard_key(text);

        if let Some(cap) = &self.key_capture {
            if cap.chord_mode && !cap.chord_keys.is_empty() {
                let cap = self.key_capture.take().unwrap();
                let (base_start, base_dur, base_track) = self
                    .clips
                    .iter()
                    .find(|c| c.id == cap.clip_id)
                    .map(|c| (c.start_ms, c.dur_ms, c.track))
                    .unwrap_or((self.playhead_ms, DEFAULT_CLIP_MS, self.active_track));

                let mut sel = HashSet::from([cap.clip_id]);
                for &extra_key in cap.chord_keys.iter().skip(1) {
                    let free_t = free_track_at(
                        &self.clips,
                        base_start,
                        base_dur,
                        base_track,
                        &HashSet::new(),
                    );
                    let id = self.alloc_clip_id();
                    self.clips.push(Clip {
                        id,
                        kind: ClipKind::Key {
                            key: Some(extra_key),
                        },
                        track: free_t,
                        start_ms: base_start,
                        dur_ms: base_dur,
                    });
                    self.tracks = self.tracks.max(free_t + 1);
                    sel.insert(id);
                }
                self.selected_ids = sel;
                let labels: Vec<String> = cap
                    .chord_keys
                    .iter()
                    .map(|&k| keyboard_key_short_label(k))
                    .collect();
                self.set_info(format!("Đã gán phím: {}", labels.join(" + ")));
                return;
            }
        }

        if self.recording.is_some() {
            if let Some(key) = key_opt {
                self.record_input_up(InputSource::Keyboard(key));
            }
        }
    }

    fn record_input_down(&mut self, source: InputSource) {
        let Some(rec) = self.recording.as_mut() else {
            return;
        };
        if rec.open_clips.contains_key(&source) {
            return;
        }
        let now = match rec.t0 {
            Some(t0) => rec
                .origin_ms
                .saturating_add(t0.elapsed().as_millis() as u64)
                .min(MAX_MS - 1),
            None => {
                rec.t0 = Some(Instant::now());
                rec.origin_ms.min(MAX_MS - 1)
            }
        };
        rec.last_now_ms = now;
        let open_ids: HashSet<u32> = rec.open_clips.values().copied().collect();

        // Find first free track at `now` that doesn't hold an open clip or overlap existing clips
        let mut track = 0_usize;
        loop {
            let busy = self.clips.iter().any(|c| {
                c.track == track
                    && (open_ids.contains(&c.id) || (c.start_ms <= now && c.end_ms() > now))
            });
            if !busy {
                break;
            }
            track += 1;
        }

        let id = self.next_clip_id;
        self.next_clip_id += 1;
        let kind = match source {
            InputSource::Keyboard(k) => ClipKind::Key { key: Some(k) },
            InputSource::MouseButton(b) => ClipKind::Mouse { button: b },
        };
        self.clips.push(Clip {
            id,
            kind,
            track,
            start_ms: now,
            dur_ms: 1,
        });
        self.tracks = self.tracks.max(track + 1);
        if let Some(rec_mut) = self.recording.as_mut() {
            rec_mut.open_clips.insert(source, id);
        }
        self.playhead_ms = now;
    }

    fn record_input_up(&mut self, source: InputSource) {
        let Some(rec) = self.recording.as_mut() else {
            return;
        };
        let Some(clip_id) = rec.open_clips.remove(&source) else {
            return;
        };
        let now = match rec.t0 {
            Some(t0) => rec
                .origin_ms
                .saturating_add(t0.elapsed().as_millis() as u64)
                .min(MAX_MS),
            None => rec.origin_ms,
        };
        rec.last_now_ms = now;
        if let Some(clip) = self.clips.iter_mut().find(|c| c.id == clip_id) {
            clip.dur_ms = now.saturating_sub(clip.start_ms).max(1);
        }
        self.playhead_ms = now;
    }

    fn nudge_horizontal(&mut self, delta_ms: i64) {
        if self.selected_ids.is_empty() {
            let next = (self.playhead_ms as i64 + delta_ms).clamp(0, MAX_MS as i64) as u64;
            self.set_playhead_ms(next);
            return;
        }
        self.push_current_undo();
        let sel = self.selected_ids.clone();
        for clip in &mut self.clips {
            if sel.contains(&clip.id) {
                let next_start = (clip.start_ms as i64 + delta_ms)
                    .clamp(0, (MAX_MS - clip.dur_ms) as i64)
                    as u64;
                clip.start_ms = next_start;
            }
        }
        resolve_overlaps(&mut self.clips, &sel, 1);
    }

    fn nudge_vertical(&mut self, delta_track: isize) {
        if self.selected_ids.is_empty() {
            return;
        }
        self.push_current_undo();
        let sel = self.selected_ids.clone();
        let min_track = self
            .clips
            .iter()
            .filter(|c| sel.contains(&c.id))
            .map(|c| c.track)
            .min()
            .unwrap_or(0) as isize;
        let effective_delta = delta_track.max(-min_track);
        for clip in &mut self.clips {
            if sel.contains(&clip.id) {
                clip.track = (clip.track as isize + effective_delta).max(0) as usize;
            }
        }
        resolve_overlaps(&mut self.clips, &sel, delta_track);
        let max_t = self.clips.iter().map(|c| c.track + 1).max().unwrap_or(0);
        self.tracks = self.tracks.max(max_t);
    }

    /// Commits edits from the Single-Clip or Multi-Clip Inspector fields.
    pub fn commit_inspector_single(
        &mut self,
        start_str: &str,
        dur_str: &str,
        end_str: &str,
        track_str: &str,
        edited_field: i32,
    ) {
        if self.selected_ids.len() != 1 || self.is_recording() {
            return;
        }
        let Some(&clip_id) = self.selected_ids.iter().next() else {
            return;
        };
        let Some(idx) = self.clips.iter().position(|c| c.id == clip_id) else {
            return;
        };

        self.push_current_undo();
        match edited_field {
            0 => {
                if let Ok(v) = start_str.trim().parse::<u64>() {
                    self.clips[idx].start_ms = v.min(MAX_MS.saturating_sub(self.clips[idx].dur_ms));
                }
            }
            1 => {
                if let Ok(v) = dur_str.trim().parse::<u64>() {
                    self.clips[idx].dur_ms = v
                        .max(1)
                        .min(MAX_MS.saturating_sub(self.clips[idx].start_ms))
                        .max(1);
                }
            }
            2 => {
                if let Ok(v) = end_str.trim().parse::<u64>() {
                    let end = v.min(MAX_MS);
                    if end > self.clips[idx].start_ms {
                        self.clips[idx].dur_ms = (end - self.clips[idx].start_ms).max(1);
                    }
                }
            }
            3 => {
                if let Ok(v) = track_str.trim().parse::<usize>() {
                    self.clips[idx].track = v.saturating_sub(1);
                }
            }
            _ => {}
        }

        let sel = HashSet::from([clip_id]);
        resolve_overlaps(&mut self.clips, &sel, 1);
        let max_t = self.clips.iter().map(|c| c.track + 1).max().unwrap_or(0);
        self.tracks = self.tracks.max(max_t);
    }

    pub fn commit_inspector_mouse_button(&mut self, button_idx: i32) {
        if self.selected_ids.len() != 1 || self.is_recording() {
            return;
        }
        let Some(btn) = MouseButton::ALL
            .get(usize::try_from(button_idx).unwrap_or(0))
            .copied()
        else {
            return;
        };
        let Some(&clip_id) = self.selected_ids.iter().next() else {
            return;
        };
        if self
            .clips
            .iter()
            .any(|c| c.id == clip_id && matches!(c.kind, ClipKind::Mouse { .. }))
        {
            self.push_current_undo();
            if let Some(clip) = self.clips.iter_mut().find(|c| c.id == clip_id) {
                clip.kind = ClipKind::Mouse { button: btn };
            }
        }
    }

    pub fn start_single_clip_key_capture(&mut self) {
        if self.selected_ids.len() != 1 || self.is_recording() {
            return;
        }
        let Some(&clip_id) = self.selected_ids.iter().next() else {
            return;
        };
        if self
            .clips
            .iter()
            .any(|c| c.id == clip_id && matches!(c.kind, ClipKind::Key { .. }))
        {
            self.key_capture = Some(KeyCaptureState {
                clip_id,
                chord_mode: false,
                chord_keys: Vec::new(),
                snapshot_pushed: false,
                created_empty_clip: false,
            });
            self.set_info("Nhấn phím mới trên bàn phím để gán cho clip (Esc để huỷ)");
        }
    }

    pub fn commit_inspector_group_move(&mut self, target_start_str: &str) {
        if self.selected_ids.len() < 2 || self.is_recording() {
            return;
        }
        let Ok(target_start) = target_start_str.trim().parse::<i64>() else {
            return;
        };
        let min_start = self
            .clips
            .iter()
            .filter(|c| self.selected_ids.contains(&c.id))
            .map(|c| c.start_ms)
            .min()
            .unwrap_or(0) as i64;
        let max_end = self
            .clips
            .iter()
            .filter(|c| self.selected_ids.contains(&c.id))
            .map(Clip::end_ms)
            .max()
            .unwrap_or(0) as i64;
        let delta = (target_start.max(0) - min_start).min(MAX_MS as i64 - max_end);

        self.push_current_undo();
        let sel = self.selected_ids.clone();
        for clip in &mut self.clips {
            if sel.contains(&clip.id) {
                clip.start_ms = (clip.start_ms as i64 + delta).max(0) as u64;
            }
        }
        resolve_overlaps(&mut self.clips, &sel, 1);
    }

    pub fn commit_inspector_group_dur(&mut self, target_dur_str: &str) {
        if self.selected_ids.len() < 2 || self.is_recording() {
            return;
        }
        let Ok(dur) = target_dur_str.trim().parse::<u64>() else {
            return;
        };
        let clamped_dur = dur.clamp(1, MAX_MS);
        self.push_current_undo();
        let sel = self.selected_ids.clone();
        for clip in &mut self.clips {
            if sel.contains(&clip.id) {
                clip.dur_ms = clamped_dur.min(MAX_MS.saturating_sub(clip.start_ms)).max(1);
            }
        }
        resolve_overlaps(&mut self.clips, &sel, 1);
    }

    /// Opens the `import.md` modal and populates it with the exported timeline sequence.
    pub fn open_io_modal(&mut self) {
        if self.is_recording() {
            return;
        }
        self.playing_preview = None;
        self.cancel_key_capture();
        self.io_text = export_clips_to_macro_text(&self.clips);
        self.refresh_io_validation();
        self.io_modal_open = true;
    }

    pub fn load_io_from_timeline(&mut self) {
        self.io_text = export_clips_to_macro_text(&self.clips);
        self.refresh_io_validation();
    }

    pub fn update_io_text(&mut self, text: String) {
        self.io_text = text;
        self.refresh_io_validation();
    }

    pub fn update_io_default_hold(&mut self, hold_str: &str) {
        if let Ok(v) = hold_str.trim().parse::<u64>() {
            self.io_default_hold_ms = v.clamp(1, MAX_MS);
            self.refresh_io_validation();
        }
    }

    pub fn refresh_io_validation(&mut self) {
        let mut dummy_id = 1;
        if self.io_text.trim().is_empty() {
            self.io_validation = UiIoValidationView {
                valid: false,
                message: "Nhập chuỗi lệnh Macro (ví dụ: {HOLDMS:100}e{WAITMS:50}{LMB})".to_string(),
                clip_count: 0,
                track_count: 0,
                total_ms: 0,
                warning_text: String::new(),
            };
            return;
        }
        match parse_macro_sequence(&self.io_text, self.io_default_hold_ms, &mut dummy_id) {
            Ok(parsed) => {
                let mut warnings = parsed.notes;
                if !parsed.system_tags.is_empty() {
                    warnings.push(format!(
                        "Thẻ hệ thống được bỏ qua khi dựng timeline: {}",
                        parsed.system_tags.join(", ")
                    ));
                }
                self.io_validation = UiIoValidationView {
                    valid: true,
                    message: "Cú pháp hợp lệ — sẵn sàng Import vào Timeline".to_string(),
                    clip_count: parsed.clips.len() as i32,
                    track_count: parsed.max_concurrent_tracks as i32,
                    total_ms: parsed.total_ms as i32,
                    warning_text: warnings.join(" · "),
                };
            }
            Err(err) => {
                self.io_validation = UiIoValidationView {
                    valid: false,
                    message: err,
                    clip_count: 0,
                    track_count: 0,
                    total_ms: 0,
                    warning_text: String::new(),
                };
            }
        }
    }

    pub fn apply_io_import(&mut self) {
        let mut next_id = self.next_clip_id;
        match parse_macro_sequence(&self.io_text, self.io_default_hold_ms, &mut next_id) {
            Ok(parsed) => {
                if parsed.clips.is_empty() {
                    self.set_error("Chuỗi không tạo ra clip nào.");
                    return;
                }
                self.next_clip_id = next_id;
                self.push_current_undo();
                if self.io_replace_mode {
                    self.clips = parsed.clips;
                    self.loop_region = None;
                    self.playhead_ms = 0;
                    self.tracks = parsed.max_concurrent_tracks.max(MIN_TRACKS);
                    self.selected_ids = self.clips.iter().map(|c| c.id).collect();
                } else {
                    let origin = self.playhead_ms;
                    let placed =
                        self.place_relative_clips(&parsed.clips, origin, self.active_track);
                    self.selected_ids = placed.into_iter().collect();
                }
                self.io_modal_open = false;
                self.fit_to_viewport();
                self.set_success(format!("Đã Import {} clip vào Timeline", self.clips.len()));
            }
            Err(err) => {
                self.io_validation.valid = false;
                self.io_validation.message = err;
            }
        }
    }

    /// Validates the current timeline and returns a ready-to-save [`MacroDefinition`].
    pub fn build_validated_macro(
        &mut self,
        next_macro_id: MacroId,
        emergency_stop: EmergencyStopHotkey,
    ) -> Result<MacroDefinition, String> {
        if let Err(err) = validate_macro_timeline(
            &self.macro_name,
            self.hotkey,
            &self.clips,
            self.loop_region,
            emergency_stop,
        ) {
            self.set_error(err.clone());
            return Err(err);
        }

        let id = self.current_macro_id.unwrap_or(next_macro_id);
        let trigger = self.hotkey.map(|source| MacroTrigger {
            source,
            mode: self.playback_mode,
        });
        let mac = MacroDefinition::new(
            id,
            self.macro_name.trim(),
            trigger,
            self.clips.clone(),
            self.effective_tracks(),
            self.loop_region,
        );
        self.current_macro_id = Some(id);
        self.set_success(format!(
            "Đã lưu Macro '{}' ({} clip)",
            mac.name,
            mac.clips.len()
        ));
        Ok(mac)
    }

    /// Computes viewport-space clip rectangles for Slint rendering.
    pub fn clip_views(&self) -> Vec<UiClipView> {
        let clashes = clashing_ids(&self.clips);
        let live_ms = self.playing_preview.as_ref().map(|p| p.now_ms);
        let mut out = Vec::with_capacity(self.clips.len());

        for clip in &self.clips {
            let x = self.ms_to_view_x(clip.start_ms);
            let w = ((clip.dur_ms as f32) * self.zoom).max(3.0);
            let y = self.track_to_view_y(clip.track) + 3.0;
            let h = TRACK_H - 6.0;

            let label = if w >= 18.0 {
                clip.short_label()
            } else {
                String::new()
            };
            let dur_text = if w >= 70.0 {
                format!("{} ms", clip.dur_ms)
            } else {
                String::new()
            };

            let kind_tag = match clip.kind {
                ClipKind::Key { key: Some(_) } => 0,
                ClipKind::Key { key: None } => 3,
                ClipKind::Mouse {
                    button: MouseButton::Right,
                } => 2,
                ClipKind::Mouse { .. } => 1,
            };

            let live = live_ms.is_some_and(|ms| clip.start_ms <= ms && ms < clip.end_ms());
            out.push(UiClipView {
                id: clip.id as i32,
                label,
                dur_text,
                x,
                y,
                width: w,
                height: h,
                kind_tag,
                selected: self.selected_ids.contains(&clip.id),
                clashing: clashes.contains(&clip.id),
                live,
            });
        }
        out
    }

    /// Computes sticky ruler major/minor tick positions visible in the current viewport.
    pub fn ruler_ticks(&self) -> Vec<UiRulerTickView> {
        let major_step = ruler_step(self.zoom);
        let minor_step = (major_step as f64 / 5.0).max(0.2);
        let min_ms = self.view_x_to_ms(HEAD_W).max(0.0);
        let max_ms = self
            .view_x_to_ms(self.viewport_w)
            .clamp(min_ms, MAX_MS as f64);

        let start_idx = (min_ms / minor_step).floor().max(0.0) as u64;
        let end_idx = (max_ms / minor_step).ceil().max(0.0) as u64;
        let mut ticks = Vec::new();
        for idx in start_idx..=end_idx.min(start_idx.saturating_add(400)) {
            let t_ms = (idx as f64) * minor_step;
            let x = HEAD_W + (t_ms as f32) * self.zoom - self.scroll_x;
            if x < HEAD_W - 2.0 || x > self.viewport_w + 2.0 {
                continue;
            }
            let is_major = idx % 5 == 0;
            let label = if is_major {
                format_ruler_ms(t_ms.round() as u64, major_step)
            } else {
                String::new()
            };
            ticks.push(UiRulerTickView { x, is_major, label });
        }
        ticks
    }

    /// Computes sticky track header rows `T1..Tn`.
    pub fn track_headers(&self) -> Vec<UiTrackHeaderView> {
        let total = self.effective_tracks();
        let mut headers = Vec::with_capacity(total);
        for t in 0..total {
            let y = self.track_to_view_y(t);
            let can_delete = t >= MIN_TRACKS && !self.clips.iter().any(|c| c.track == t);
            headers.push(UiTrackHeaderView {
                index: t as i32,
                label: format!("T{}", t + 1),
                y,
                active: t == self.active_track,
                can_delete,
            });
        }
        headers
    }

    /// Computes Clip Inspector properties for either single-clip or multi-clip selection.
    pub fn inspector_view(&self) -> UiInspectorView {
        if self.selected_ids.is_empty() {
            return UiInspectorView::default();
        }
        if self.selected_ids.len() == 1 {
            let Some(&clip_id) = self.selected_ids.iter().next() else {
                return UiInspectorView::default();
            };
            let Some(clip) = self.clips.iter().find(|c| c.id == clip_id) else {
                return UiInspectorView::default();
            };
            let (is_key_clip, key_label, mouse_button_idx) = match clip.kind {
                ClipKind::Key { key: Some(k) } => (true, k.display_name(), 0),
                ClipKind::Key { key: None } => (true, "Chờ chọn phím (?)".to_string(), 0),
                ClipKind::Mouse { button } => {
                    let idx = MouseButton::ALL
                        .iter()
                        .position(|&b| b == button)
                        .unwrap_or(0) as i32;
                    (false, String::new(), idx)
                }
            };
            return UiInspectorView {
                visible: true,
                multi_mode: false,
                title: format!("Clip: {}", clip.full_label()),
                is_key_clip,
                key_label,
                mouse_button_idx,
                start_ms_text: clip.start_ms.to_string(),
                dur_ms_text: clip.dur_ms.to_string(),
                end_ms_text: clip.end_ms().to_string(),
                track_text: (clip.track + 1).to_string(),
                group_start_text: String::new(),
                group_dur_text: String::new(),
            };
        }

        let selected: Vec<&Clip> = self
            .clips
            .iter()
            .filter(|c| self.selected_ids.contains(&c.id))
            .collect();
        let min_start = selected.iter().map(|c| c.start_ms).min().unwrap_or(0);
        let max_end = selected.iter().map(|c| c.end_ms()).max().unwrap_or(0);
        UiInspectorView {
            visible: true,
            multi_mode: true,
            title: format!(
                "{} clip đã chọn · {} ms → {} ms",
                selected.len(),
                min_start,
                max_end
            ),
            is_key_clip: false,
            key_label: String::new(),
            mouse_button_idx: 0,
            start_ms_text: String::new(),
            dur_ms_text: String::new(),
            end_ms_text: String::new(),
            track_text: String::new(),
            group_start_text: min_start.to_string(),
            group_dur_text: selected
                .first()
                .map(|c| c.dur_ms.to_string())
                .unwrap_or_else(|| "100".to_string()),
        }
    }

    pub fn compiled_preview_text(&self) -> String {
        let steps = compile_clips(&self.clips, self.loop_region);
        format_compiled_preview(&steps)
    }

    pub fn loop_status_text(&self) -> String {
        match self.loop_region {
            Some(lp) => format!(
                "Vùng lặp: {} → {} ms ({} ms)",
                lp.start_ms,
                lp.end_ms,
                lp.end_ms.saturating_sub(lp.start_ms)
            ),
            None => "Vùng lặp: Không lặp".to_string(),
        }
    }

    pub fn zoom_label(&self) -> String {
        let ms_per_100px = (100.0 / self.zoom.max(ZOOM_MIN)).round() as u64;
        format!("{ms_per_100px} ms / 100 px")
    }

    pub fn ms_to_view_x(&self, ms: u64) -> f32 {
        HEAD_W + (ms as f32) * self.zoom - self.scroll_x
    }

    pub fn view_x_to_ms(&self, vx: f32) -> f64 {
        let inner_x = vx + self.scroll_x - HEAD_W;
        (inner_x / self.zoom) as f64
    }

    pub fn track_to_view_y(&self, track: usize) -> f32 {
        RULER_H + (track as f32) * TRACK_H - self.scroll_y
    }

    pub fn view_y_to_track(&self, vy: f32) -> isize {
        let inner_y = vy + self.scroll_y - RULER_H;
        (inner_y / TRACK_H).floor() as isize
    }

    pub fn playhead_view_x(&self) -> f32 {
        self.ms_to_view_x(self.playhead_ms)
    }

    pub fn snap_guide_view_x(&self) -> f32 {
        self.snap_guide_ms
            .map(|ms| self.ms_to_view_x(ms))
            .unwrap_or(-100.0)
    }

    pub fn max_scroll_x(&self) -> f32 {
        let visible_ms = (self.viewport_w - HEAD_W).max(0.0) / self.zoom;
        let span_ms = (self.content_end_ms() as f32 + 1000.0)
            .max(self.playhead_ms as f32 + 500.0)
            .max(visible_ms);
        let virtual_w = HEAD_W + span_ms * self.zoom;
        (virtual_w - self.viewport_w).max(0.0)
    }

    pub fn max_scroll_y(&self) -> f32 {
        let virtual_h = RULER_H + (self.effective_tracks() as f32) * TRACK_H;
        (virtual_h - self.viewport_h).max(0.0)
    }

    fn clamp_scroll(&mut self) {
        self.scroll_x = self.scroll_x.clamp(0.0, self.max_scroll_x());
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y());
    }

    fn hit_test_clip(&self, vx: f32, vy: f32) -> Option<usize> {
        let mut fallback: Option<usize> = None;
        for idx in (0..self.clips.len()).rev() {
            let clip = &self.clips[idx];
            let cx = self.ms_to_view_x(clip.start_ms);
            let cw = ((clip.dur_ms as f32) * self.zoom).max(3.0);
            let cy = self.track_to_view_y(clip.track) + 3.0;
            let ch = TRACK_H - 6.0;
            if vx >= cx && vx <= cx + cw && vy >= cy && vy <= cy + ch {
                if self.selected_ids.contains(&clip.id) {
                    return Some(idx);
                }
                if fallback.is_none() {
                    fallback = Some(idx);
                }
            }
        }
        fallback
    }

    fn update_hover_cursor(&mut self, vx: f32, vy: f32) {
        if vx <= HEAD_W || vy <= RULER_H {
            self.hover_resize = false;
            return;
        }
        if let Some(idx) = self.hit_test_clip(vx, vy) {
            let clip = &self.clips[idx];
            let cx = self.ms_to_view_x(clip.start_ms);
            let cw = ((clip.dur_ms as f32) * self.zoom).max(3.0);
            self.hover_resize =
                cw > EDGE_PX * 3.0 && (vx - cx <= EDGE_PX || (cx + cw) - vx <= EDGE_PX);
        } else {
            self.hover_resize = false;
        }
    }

    fn cancel_key_capture(&mut self) {
        if let Some(cap) = self.key_capture.take() {
            if cap.created_empty_clip {
                self.clips.retain(|c| c.id != cap.clip_id);
                self.selected_ids.remove(&cap.clip_id);
                self.undo_stack.pop();
            }
        }
    }

    fn alloc_clip_id(&mut self) -> u32 {
        let id = self.next_clip_id;
        self.next_clip_id += 1;
        id
    }

    fn set_info(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_is_error = false;
        self.status_is_success = false;
    }

    fn set_error(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_is_error = true;
        self.status_is_success = false;
    }

    fn set_success(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_is_error = false;
        self.status_is_success = true;
    }
}

const fn mouse_button_from_code(code: i32) -> Option<MouseButton> {
    match code {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Right),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::XButton1),
        4 => Some(MouseButton::XButton2),
        _ => None,
    }
}

/// Converts a Slint `KeyEvent.text` string to a domain [`KeyboardKey`].
pub fn slint_text_to_keyboard_key(text: &str) -> Option<KeyboardKey> {
    let ch = text.chars().next()?;
    match ch {
        '\u{0008}' => Some(KeyboardKey::Backspace),
        '\u{0009}' => Some(KeyboardKey::Tab),
        '\u{000a}' | '\r' => Some(KeyboardKey::Enter),
        '\u{001b}' => Some(KeyboardKey::Escape),
        '\u{007f}' => Some(KeyboardKey::Delete),
        '\u{0010}' => Some(KeyboardKey::LeftShift),
        '\u{0015}' => Some(KeyboardKey::RightShift),
        '\u{0011}' => Some(KeyboardKey::LeftCtrl),
        '\u{0016}' => Some(KeyboardKey::RightCtrl),
        '\u{0012}' => Some(KeyboardKey::LeftAlt),
        '\u{0013}' => Some(KeyboardKey::RightAlt),
        '\u{0014}' => Some(KeyboardKey::CapsLock),
        '\u{0017}' => Some(KeyboardKey::LeftMeta),
        '\u{0018}' => Some(KeyboardKey::RightMeta),
        '\u{0020}' => Some(KeyboardKey::Space),
        '\u{F700}' => Some(KeyboardKey::ArrowUp),
        '\u{F701}' => Some(KeyboardKey::ArrowDown),
        '\u{F702}' => Some(KeyboardKey::ArrowLeft),
        '\u{F703}' => Some(KeyboardKey::ArrowRight),
        '\u{F704}' => Some(KeyboardKey::F1),
        '\u{F705}' => Some(KeyboardKey::F2),
        '\u{F706}' => Some(KeyboardKey::F3),
        '\u{F707}' => Some(KeyboardKey::F4),
        '\u{F708}' => Some(KeyboardKey::F5),
        '\u{F709}' => Some(KeyboardKey::F6),
        '\u{F70A}' => Some(KeyboardKey::F7),
        '\u{F70B}' => Some(KeyboardKey::F8),
        '\u{F70C}' => Some(KeyboardKey::F9),
        '\u{F70D}' => Some(KeyboardKey::F10),
        '\u{F70E}' => Some(KeyboardKey::F11),
        '\u{F70F}' => Some(KeyboardKey::F12),
        '\u{F710}' => Some(KeyboardKey::F13),
        '\u{F711}' => Some(KeyboardKey::F14),
        '\u{F712}' => Some(KeyboardKey::F15),
        '\u{F713}' => Some(KeyboardKey::F16),
        '\u{F714}' => Some(KeyboardKey::F17),
        '\u{F715}' => Some(KeyboardKey::F18),
        '\u{F716}' => Some(KeyboardKey::F19),
        '\u{F717}' => Some(KeyboardKey::F20),
        '\u{F718}' => Some(KeyboardKey::F21),
        '\u{F719}' => Some(KeyboardKey::F22),
        '\u{F71A}' => Some(KeyboardKey::F23),
        '\u{F71B}' => Some(KeyboardKey::F24),
        '\u{F727}' => Some(KeyboardKey::Insert),
        '\u{F729}' => Some(KeyboardKey::Home),
        '\u{F72B}' => Some(KeyboardKey::End),
        '\u{F72C}' => Some(KeyboardKey::PageUp),
        '\u{F72D}' => Some(KeyboardKey::PageDown),
        '\u{F72F}' => Some(KeyboardKey::ScrollLock),
        '\u{F730}' => Some(KeyboardKey::Pause),
        '\u{F731}' => Some(KeyboardKey::PrintScreen),
        '\u{F735}' => Some(KeyboardKey::ContextMenu),
        c => match c.to_ascii_lowercase() {
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
            '0' | ')' => Some(KeyboardKey::Digit0),
            '1' | '!' => Some(KeyboardKey::Digit1),
            '2' | '@' => Some(KeyboardKey::Digit2),
            '3' | '#' => Some(KeyboardKey::Digit3),
            '4' | '$' => Some(KeyboardKey::Digit4),
            '5' | '%' => Some(KeyboardKey::Digit5),
            '6' | '^' => Some(KeyboardKey::Digit6),
            '7' | '&' => Some(KeyboardKey::Digit7),
            '8' | '*' => Some(KeyboardKey::Digit8),
            '9' | '(' => Some(KeyboardKey::Digit9),
            '`' | '~' => Some(KeyboardKey::Grave),
            '-' | '_' => Some(KeyboardKey::Minus),
            '=' | '+' => Some(KeyboardKey::Equal),
            '[' | '{' => Some(KeyboardKey::LeftBracket),
            ']' | '}' => Some(KeyboardKey::RightBracket),
            '\\' | '|' => Some(KeyboardKey::Backslash),
            ';' | ':' => Some(KeyboardKey::Semicolon),
            '\'' | '"' => Some(KeyboardKey::Quote),
            ',' | '<' => Some(KeyboardKey::Comma),
            '.' | '>' => Some(KeyboardKey::Period),
            '/' | '?' => Some(KeyboardKey::Slash),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_undo_redo_and_chord_key_capture_work() {
        let mut editor = MacroEditorState::default();
        editor.insert_mouse_clip(MouseButton::Left);
        assert_eq!(editor.clips.len(), 1);
        assert!(editor.can_undo());

        editor.undo();
        assert!(editor.clips.is_empty());
        assert!(editor.can_redo());

        editor.redo();
        assert_eq!(editor.clips.len(), 1);

        // Insert key clip and capture Ctrl + C chord
        editor.set_playhead_ms(200);
        editor.insert_key_clip_and_capture();
        assert_eq!(editor.clips.len(), 2);
        editor.on_key_pressed("\u{0011}", false, true, false, false);
        editor.on_key_pressed("c", false, true, false, false);
        editor.on_key_released("c");
        assert_eq!(editor.clips.len(), 3);
        assert!(editor.clips.iter().any(|c| c.kind
            == ClipKind::Key {
                key: Some(KeyboardKey::LeftCtrl)
            }));
        assert!(editor.clips.iter().any(|c| c.kind
            == ClipKind::Key {
                key: Some(KeyboardKey::KeyC)
            }));
    }

    #[test]
    fn emergency_stop_and_hotkey_mouse_restrictions_are_enforced() {
        let mut editor = MacroEditorState {
            capturing_hotkey: true,
            ..Default::default()
        };

        // Left mouse button rejected
        editor.on_pointer_down(100.0, 100.0, 0, false, false, false);
        assert!(editor.capturing_hotkey);
        assert!(editor.hotkey.is_none());
        assert!(editor.status_is_error);

        // Side button M3 accepted
        editor.on_pointer_down(100.0, 100.0, 3, false, false, false);
        assert!(!editor.capturing_hotkey);
        assert_eq!(
            editor.hotkey,
            Some(InputSource::MouseButton(MouseButton::XButton1))
        );

        // Start playback and verify stop_all_active_modes stops it cleanly
        editor.insert_mouse_clip(MouseButton::Right);
        editor.toggle_playback();
        assert!(editor.is_playing());
        editor.stop_all_active_modes();
        assert!(!editor.is_playing());
    }
}
