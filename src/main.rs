#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ctt_app::app::{CttApp, NavTab};
use ctt_app::domain::keys::{InputSource, KeyboardKey, MouseButton, MouseWheel};
use ctt_app::domain::macros::{MacroPlaybackMode, MAX_MS};
use ctt_app::engine::command::EngineStatus;
use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

slint::include_modules!();

/// Populates static source-input, keyboard, and mouse target catalogs once at startup.
fn init_catalogs(window: &AppWindow) {
    let source_names: Vec<SharedString> = InputSource::all_standard()
        .into_iter()
        .map(|s| s.display_name().into())
        .collect();
    window.set_source_key_names(ModelRc::new(VecModel::from(source_names)));

    let key_names: Vec<SharedString> = KeyboardKey::ALL_STANDARD
        .iter()
        .map(|k| k.display_name().into())
        .collect();
    window.set_keyboard_key_names(ModelRc::new(VecModel::from(key_names)));

    let mouse_btn_names: Vec<SharedString> = MouseButton::ALL
        .iter()
        .map(|b| b.display_name().into())
        .collect();
    window.set_mouse_button_names(ModelRc::new(VecModel::from(mouse_btn_names)));

    let wheel_names: Vec<SharedString> = MouseWheel::ALL
        .iter()
        .map(|w| w.display_name().into())
        .collect();
    window.set_mouse_wheel_names(ModelRc::new(VecModel::from(wheel_names)));
}

/// Pushes current macro timeline, inspector, and modal validation state into Slint.
fn sync_macro_ui(window: &AppWindow, app: &CttApp, sync_text_inputs: bool) {
    let saved: Vec<UiSavedMacroItem> = app
        .saved_macro_views()
        .into_iter()
        .map(|m| UiSavedMacroItem {
            id: m.id,
            name: m.name.into(),
            trigger_label: m.trigger_label.into(),
            clip_count: m.clip_count,
            total_ms: m.total_ms,
        })
        .collect();
    window.set_macro_saved_items(ModelRc::new(VecModel::from(saved)));

    let me = app.macro_editor();
    window.set_macro_editor_open(me.editor_open);
    window.set_macro_clip_count(me.clips.len() as i32);
    window.set_macro_selected_count(me.selected_ids.len() as i32);
    window.set_macro_total_ms(me.content_end_ms() as i32);
    window.set_macro_playhead_ms(me.playhead_ms as i32);
    window.set_macro_playhead_input_text(me.playhead_ms.to_string().into());
    window.set_macro_active_track_label(format!("T{}", me.active_track + 1).into());
    window.set_macro_loop_status_text(me.loop_status_text().into());
    window.set_macro_zoom_label(me.zoom_label().into());
    window.set_macro_can_undo(me.can_undo());
    window.set_macro_can_redo(me.can_redo());
    window.set_macro_can_razor(me.can_razor());
    window.set_macro_has_loop(me.loop_region.is_some());
    window.set_macro_snap_enabled(me.snap_enabled);
    window.set_macro_is_playing(me.is_playing());
    window.set_macro_is_recording(me.is_recording());
    window.set_macro_is_capturing_key(me.is_capturing_clip_key());
    window.set_macro_is_capturing_hotkey(me.capturing_hotkey);
    window.set_macro_capture_banner(me.capture_banner_text().into());
    window.set_macro_hover_resize(me.hover_resize);

    let clips: Vec<UiClipItem> = me
        .clip_views()
        .into_iter()
        .map(|c| UiClipItem {
            id: c.id,
            label: c.label.into(),
            dur_text: c.dur_text.into(),
            x: c.x,
            y: c.y,
            width: c.width,
            height: c.height,
            kind_tag: c.kind_tag,
            selected: c.selected,
            clashing: c.clashing,
            live: c.live,
        })
        .collect();
    window.set_macro_clips(ModelRc::new(VecModel::from(clips)));

    let ticks: Vec<UiRulerTick> = me
        .ruler_ticks()
        .into_iter()
        .map(|t| UiRulerTick {
            x: t.x,
            is_major: t.is_major,
            label: t.label.into(),
        })
        .collect();
    window.set_macro_ruler_ticks(ModelRc::new(VecModel::from(ticks)));

    let headers: Vec<UiTrackHeader> = me
        .track_headers()
        .into_iter()
        .map(|h| UiTrackHeader {
            index: h.index,
            label: h.label.into(),
            y: h.y,
            active: h.active,
            can_delete: h.can_delete,
        })
        .collect();
    window.set_macro_track_headers(ModelRc::new(VecModel::from(headers)));

    window.set_macro_playhead_x(me.playhead_view_x());
    if let Some(lp) = me.loop_region {
        let lx = me.ms_to_view_x(lp.start_ms);
        let lw = ((lp.end_ms.saturating_sub(lp.start_ms) as f32) * me.zoom).max(2.0);
        window.set_macro_has_loop_overlay(true);
        window.set_macro_loop_x(lx);
        window.set_macro_loop_width(lw);
    } else {
        window.set_macro_has_loop_overlay(false);
        window.set_macro_loop_x(0.0);
        window.set_macro_loop_width(0.0);
    }

    if me.snap_guide_ms.is_some() {
        window.set_macro_show_snap_guide(true);
        window.set_macro_snap_guide_x(me.snap_guide_view_x());
    } else {
        window.set_macro_show_snap_guide(false);
    }

    if let Some((mx, my, mw, mh)) = me.marquee_rect {
        window.set_macro_show_marquee(true);
        window.set_macro_marquee_x(mx);
        window.set_macro_marquee_y(my);
        window.set_macro_marquee_width(mw);
        window.set_macro_marquee_height(mh);
    } else {
        window.set_macro_show_marquee(false);
    }

    let insp = me.inspector_view();
    window.set_macro_inspector_visible(insp.visible);
    window.set_macro_inspector_multi(insp.multi_mode);
    window.set_macro_inspector_title(insp.title.into());
    window.set_macro_inspector_is_key(insp.is_key_clip);
    window.set_macro_inspector_key_label(insp.key_label.into());
    window.set_macro_inspector_mouse_idx(insp.mouse_button_idx);
    window.set_macro_inspector_start_text(insp.start_ms_text.into());
    window.set_macro_inspector_dur_text(insp.dur_ms_text.into());
    window.set_macro_inspector_end_text(insp.end_ms_text.into());
    window.set_macro_inspector_track_text(insp.track_text.into());
    window.set_macro_inspector_group_start_text(insp.group_start_text.into());
    window.set_macro_inspector_group_dur_text(insp.group_dur_text.into());

    if sync_text_inputs {
        window.set_macro_name(me.macro_name.clone().into());
        window.set_macro_io_text(me.io_text.clone().into());
        window.set_macro_gap_input_text(me.gap_input_ms.to_string().into());
        window.set_macro_io_default_hold_text(me.io_default_hold_ms.to_string().into());
    }

    window.set_macro_has_hotkey(me.hotkey.is_some());
    window.set_macro_hotkey_label(
        me.hotkey
            .map(|h| h.display_name())
            .unwrap_or_else(|| "Chưa gán (tuỳ chọn)".to_string())
            .into(),
    );
    window.set_macro_playback_mode_idx(me.playback_mode.to_ui_index());
    window.set_macro_compiled_preview(me.compiled_preview_text().into());
    window.set_macro_status_message(me.status_message.clone().into());
    window.set_macro_status_is_error(me.status_is_error);
    window.set_macro_status_is_success(me.status_is_success);

    window.set_macro_io_modal_open(me.io_modal_open);
    window.set_macro_io_replace_mode(me.io_replace_mode);
    window.set_macro_io_valid(me.io_validation.valid);
    window.set_macro_io_validation_message(me.io_validation.message.clone().into());
    window.set_macro_io_clip_count(me.io_validation.clip_count);
    window.set_macro_io_track_count(me.io_validation.track_count);
    window.set_macro_io_total_ms(me.io_validation.total_ms);
    window.set_macro_io_warning_text(me.io_validation.warning_text.clone().into());
}

/// Pushes current [`CttApp`] domain and engine state into the Slint [`AppWindow`] properties.
fn sync_ui(window: &AppWindow, app: &CttApp) {
    window.set_active_tab(app.active_tab().as_index());

    let profile_names: Vec<SharedString> =
        app.profile_names().into_iter().map(Into::into).collect();
    window.set_profile_names(ModelRc::new(VecModel::from(profile_names)));

    let active_idx = i32::try_from(app.active_profile_index()).unwrap_or(0);
    window.set_has_profile(app.has_active_profile());
    window.set_active_profile_index(active_idx);
    window.set_active_profile_name(app.active_profile_name().into());

    let bindings: Vec<BindingItem> = app
        .active_bindings()
        .iter()
        .map(|entry| BindingItem {
            action: entry.action.as_str().into(),
            shortcut: entry.shortcut.as_str().into(),
        })
        .collect();
    window.set_bindings(ModelRc::new(VecModel::from(bindings)));
    window.set_binding_summary(app.binding_summary().into());

    let rule_items: Vec<BindingRuleItem> = app
        .domain_rule_views()
        .into_iter()
        .map(|rule| BindingRuleItem {
            source_key: rule.source_label.into(),
            target_action: rule.target_label.into(),
            label: rule.label.into(),
            enabled: rule.enabled,
        })
        .collect();
    window.set_rule_items(ModelRc::new(VecModel::from(rule_items)));

    window.set_validation_error(app.validation_error().into());
    window.set_validation_warning(app.validation_warnings().join(" | ").into());

    let engine_status = app.engine_status();
    window.set_engine_status_text(engine_status.display_name().into());
    window.set_mapping_active(engine_status == EngineStatus::Active);
    window.set_emergency_stopped(engine_status == EngineStatus::EmergencyStopped);
    window.set_emergency_stop_hotkey(app.emergency_stop_label().into());

    window.set_counter(app.counter());
    window.set_status_text(app.status().into());
    window.set_search_query(app.search_query().into());

    sync_macro_ui(window, app, false);
}

fn bind_macro_callbacks(app_window: &AppWindow, state: &Rc<RefCell<CttApp>>) {
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_new(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().reset_new_macro();
                sync_macro_ui(&window, &s, true);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_close_editor(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().close_editor();
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_open_io(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().open_io_modal();
                sync_macro_ui(&window, &s, true);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_save(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                let _ = s.save_current_macro();
                sync_ui(&window, &s);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_load_saved(move |idx| {
            if let Some(window) = app_weak.upgrade() {
                if let Ok(index) = usize::try_from(idx) {
                    let mut s = state_clone.borrow_mut();
                    if s.load_saved_macro(index) {
                        sync_macro_ui(&window, &s, true);
                    }
                }
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_delete_saved(move |idx| {
            if let Some(window) = app_weak.upgrade() {
                if let Ok(index) = usize::try_from(idx) {
                    let mut s = state_clone.borrow_mut();
                    if s.delete_saved_macro(index) {
                        window.set_macro_selected_saved_idx(0);
                        sync_ui(&window, &s);
                    }
                }
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_toolbar_action(move |action| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                let me = s.macro_editor_mut();
                match action.as_str() {
                    "undo" => me.undo(),
                    "redo" => me.redo(),
                    "cut" => me.cut_selected(),
                    "copy" => me.copy_selected(),
                    "paste" => me.paste_at_playhead(),
                    "dup" => me.duplicate_selected(),
                    "razor" => me.razor_at_playhead(),
                    "delete" => me.delete_selected(),
                    "clear" => me.clear_timeline(),
                    "loop_in" => me.set_loop_in(),
                    "loop_out" => me.set_loop_out(),
                    "loop_off" => me.clear_loop(),
                    "play" => me.toggle_playback(),
                    "record" => me.toggle_recording(),
                    "snap" => me.toggle_snap(),
                    "zoom_out" => me.set_zoom(me.zoom * 0.8, None),
                    "zoom_in" => me.set_zoom(me.zoom * 1.25, None),
                    "fit" => me.fit_to_viewport(),
                    "add_track" => me.add_track(),
                    _ => {}
                }
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_insert_key(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().insert_key_clip_and_capture();
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_insert_mouse(move |btn_idx| {
            if let Some(window) = app_weak.upgrade() {
                if let Some(&btn) = MouseButton::ALL.get(usize::try_from(btn_idx).unwrap_or(0)) {
                    let mut s = state_clone.borrow_mut();
                    s.macro_editor_mut().insert_mouse_clip(btn);
                    sync_macro_ui(&window, &s, false);
                }
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_insert_gap(move |gap_str| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                if let Ok(ms) = gap_str.trim().parse::<u64>() {
                    s.macro_editor_mut().gap_input_ms = ms.clamp(1, MAX_MS);
                }
                s.macro_editor_mut().insert_gap_at_playhead();
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_set_playhead_text(move |txt| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                if let Ok(ms) = txt.trim().parse::<u64>() {
                    s.macro_editor_mut().set_playhead_ms(ms);
                }
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_canvas_resized(move |w, h| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().set_viewport_size(w, h);
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_canvas_pointer_down(move |vx, vy, btn, ctrl, shift, alt| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut()
                    .on_pointer_down(vx, vy, btn, ctrl, shift, alt);
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_canvas_pointer_move(move |vx, vy, alt| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().on_pointer_move(vx, vy, alt);
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_canvas_pointer_up(move |btn, ctrl, shift, alt| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().on_pointer_up(btn, ctrl, shift, alt);
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_canvas_double_click(move |vx, vy, alt| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().on_double_click(vx, vy, alt);
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_canvas_wheel(move |vx, dx, dy, ctrl, shift| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().on_wheel(vx, dx, dy, ctrl, shift);
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_canvas_key_pressed(move |text, repeat, ctrl, shift, alt| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                let save_requested =
                    s.macro_editor_mut()
                        .on_key_pressed(text.as_str(), repeat, ctrl, shift, alt);
                if save_requested {
                    let _ = s.save_current_macro();
                    sync_ui(&window, &s);
                } else {
                    sync_macro_ui(&window, &s, false);
                }
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_canvas_key_released(move |text| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().on_key_released(text.as_str());
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_inspector_commit_single(move |st, du, en, tr, field| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().commit_inspector_single(
                    st.as_str(),
                    du.as_str(),
                    en.as_str(),
                    tr.as_str(),
                    field,
                );
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_inspector_select_mouse(move |btn_idx| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().commit_inspector_mouse_button(btn_idx);
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_inspector_capture_key(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().start_single_clip_key_capture();
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_inspector_commit_group_start(move |val| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut()
                    .commit_inspector_group_move(val.as_str());
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_inspector_commit_group_dur(move |val| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut()
                    .commit_inspector_group_dur(val.as_str());
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let state_clone = state.clone();
        app_window.on_macro_name_edited(move |txt| {
            let mut s = state_clone.borrow_mut();
            s.macro_editor_mut().macro_name = txt.to_string();
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_toggle_hotkey_capture(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                let next = !s.macro_editor().capturing_hotkey;
                s.macro_editor_mut().capturing_hotkey = next;
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_clear_hotkey(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().hotkey = None;
                s.macro_editor_mut().capturing_hotkey = false;
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_select_playback_mode(move |idx| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().playback_mode = MacroPlaybackMode::from_ui_index(idx);
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_io_close(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().io_modal_open = false;
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_io_set_replace_mode(move |rep| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().io_replace_mode = rep;
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_io_default_hold_changed(move |txt| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().update_io_default_hold(txt.as_str());
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_io_load_from_timeline(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().load_io_from_timeline();
                sync_macro_ui(&window, &s, true);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_io_text_edited(move |txt| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().update_io_text(txt.to_string());
                sync_macro_ui(&window, &s, false);
            }
        });
    }

    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_macro_io_apply_import(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.macro_editor_mut().apply_io_import();
                sync_macro_ui(&window, &s, true);
            }
        });
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let app_window = AppWindow::new()?;
    let state = Rc::new(RefCell::new(CttApp::new()));

    init_catalogs(&app_window);
    {
        let s = state.borrow();
        sync_ui(&app_window, &s);
        sync_macro_ui(&app_window, &s, true);
    }

    // Handle sidebar tab selection
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_select_tab(move |tab_idx| {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.set_active_tab(NavTab::from_index(tab_idx));
                sync_ui(&window, &s);
            }
        });
    }

    // Handle profile selection from dropdown
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_select_profile(move |idx| {
            if let Some(window) = app_weak.upgrade() {
                if let Ok(index) = usize::try_from(idx) {
                    let mut s = state_clone.borrow_mut();
                    if s.select_profile(index) {
                        sync_ui(&window, &s);
                    }
                }
            }
        });
    }

    // Handle creating a new profile
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_create_profile(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.create_profile();
                sync_ui(&window, &s);
            }
        });
    }

    // Handle deleting the active profile
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_delete_profile(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.delete_active_profile();
                sync_ui(&window, &s);
            }
        });
    }

    // Handle creating/updating a key binding rule
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_apply_binding(
            move |source_idx,
                  target_kind,
                  target_key_idx,
                  ctrl,
                  shift,
                  alt,
                  meta,
                  mouse_btn_idx,
                  wheel_idx,
                  label| {
                if let Some(window) = app_weak.upgrade() {
                    let mut s = state_clone.borrow_mut();
                    let src = usize::try_from(source_idx).unwrap_or(0);
                    let kind = usize::try_from(target_kind).unwrap_or(0);
                    let tkey = usize::try_from(target_key_idx).unwrap_or(0);
                    let mbtn = usize::try_from(mouse_btn_idx).unwrap_or(0);
                    let wheel = usize::try_from(wheel_idx).unwrap_or(0);

                    if let Err(err_msg) = s.apply_binding_selection(
                        src,
                        kind,
                        tkey,
                        ctrl,
                        shift,
                        alt,
                        meta,
                        mbtn,
                        wheel,
                        label.as_str(),
                    ) {
                        s.set_status(format!("Binding rejected: {err_msg}"));
                    }
                    sync_ui(&window, &s);
                }
            },
        );
    }

    // Handle removing an existing key binding rule
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_remove_binding(move |row_idx| {
            if let Some(window) = app_weak.upgrade() {
                if let Ok(index) = usize::try_from(row_idx) {
                    let mut s = state_clone.borrow_mut();
                    if let Some(rule) = s.domain_rule_views().get(index).cloned() {
                        s.remove_binding(rule.source_key);
                        sync_ui(&window, &s);
                    }
                }
            }
        });
    }

    // Handle toggling mapping active/paused state
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_toggle_mapping(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                let next = !s.is_mapping_enabled();
                s.set_mapping_enabled(next);
                sync_ui(&window, &s);
            }
        });
    }

    // Handle emergency stop button
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_emergency_stop(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.trigger_emergency_stop();
                sync_ui(&window, &s);
            }
        });
    }

    // Handle counter increment callback
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        app_window.on_increment_counter(move || {
            if let Some(window) = app_weak.upgrade() {
                let mut s = state_clone.borrow_mut();
                s.increment();
                sync_ui(&window, &s);
            }
        });
    }

    // Handle quit callback
    {
        let app_weak = app_window.as_weak();
        app_window.on_quit_requested(move || {
            if let Some(window) = app_weak.upgrade() {
                let _ = window.hide();
            }
        });
    }

    bind_macro_callbacks(&app_window, &state);

    // 60 FPS animation timer for smooth Preview Playback & Live Recording updates
    let anim_timer = Timer::default();
    {
        let app_weak = app_window.as_weak();
        let state_clone = state.clone();
        anim_timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
            let mut s = state_clone.borrow_mut();
            if s.macro_editor_mut().tick_animation() {
                if let Some(window) = app_weak.upgrade() {
                    sync_macro_ui(&window, &s, false);
                }
            }
        });
    }

    app_window.run()
}
