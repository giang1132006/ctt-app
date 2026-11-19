#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ctt_app::app::{CttApp, NavTab};
use ctt_app::domain::keys::{InputSource, KeyboardKey, MouseButton, MouseWheel};
use ctt_app::engine::command::EngineStatus;
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

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
}

fn main() -> Result<(), slint::PlatformError> {
    let app_window = AppWindow::new()?;
    let state = Rc::new(RefCell::new(CttApp::new()));

    init_catalogs(&app_window);
    {
        let s = state.borrow();
        sync_ui(&app_window, &s);
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

    app_window.run()
}
