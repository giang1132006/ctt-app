//! Top-level UI application coordinator bridging Slint views with domain rules and the remapping engine.

use crate::domain::config::AppConfig;
use crate::domain::keys::{InputSource, KeyboardKey, ModifierMask, MouseButton, MouseWheel};
use crate::domain::mapping::MappingTarget;
use crate::domain::profile::Profile as DomainProfile;
use crate::domain::validation::{validate_profile_rules, MappingError, MappingWarning};
use crate::engine::command::EngineStatus;
use crate::engine::remapper::RemapEngine;
use crate::platform::mock::MockInputSynthesizer;

/// Primary sidebar navigation tabs matching the application layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTab {
    Overview,
    Macros,
    Binding,
}

impl NavTab {
    /// Converts a UI tab index into a [`NavTab`], defaulting to [`NavTab::Overview`].
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Macros,
            2 => Self::Binding,
            _ => Self::Overview,
        }
    }

    /// Returns the zero-based tab index expected by the Slint UI.
    pub fn as_index(self) -> i32 {
        match self {
            Self::Overview => 0,
            Self::Macros => 1,
            Self::Binding => 2,
        }
    }
}

/// A single action-to-shortcut entry displayed in the overview binding table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingEntry {
    pub action: String,
    pub shortcut: String,
}

impl BindingEntry {
    /// Creates a new [`BindingEntry`] from an action label and shortcut string.
    pub fn new(action: impl Into<String>, shortcut: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            shortcut: shortcut.into(),
        }
    }
}

/// UI projection of a [`DomainProfile`] used by overview views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub bindings: Vec<BindingEntry>,
}

impl Profile {
    fn from_domain(domain: &DomainProfile) -> Self {
        Self {
            name: domain.name.clone(),
            bindings: domain
                .rules
                .iter()
                .map(|rule| BindingEntry::new(rule.label.clone(), rule.target.display_name()))
                .collect(),
        }
    }
}

/// Detailed rule projection rendered in the interactive Key Binding editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainRuleView {
    pub id: u32,
    pub source_key: InputSource,
    pub source_label: String,
    pub target_label: String,
    pub label: String,
    pub enabled: bool,
}

/// Top-level application state coordinating navigation, profiles, bindings, and the remap engine.
#[derive(Debug)]
pub struct CttApp {
    active_tab: NavTab,
    config: AppConfig,
    domain_profiles: Vec<DomainProfile>,
    ui_profiles: Vec<Profile>,
    active_profile_index: usize,
    next_profile_number: usize,
    next_profile_id: u32,
    engine: RemapEngine,
    synthesizer: MockInputSynthesizer,
    validation_error: String,
    validation_warnings: Vec<String>,
    search_query: String,
    counter: i32,
    status: String,
}

impl Default for CttApp {
    fn default() -> Self {
        let config = AppConfig::default();
        let engine = RemapEngine::new(config.emergency_stop);
        let synthesizer = MockInputSynthesizer::new();

        Self {
            active_tab: NavTab::Overview,
            config,
            domain_profiles: Vec::new(),
            ui_profiles: Vec::new(),
            active_profile_index: 0,
            next_profile_number: 1,
            next_profile_id: 1,
            engine,
            synthesizer,
            validation_error: String::new(),
            validation_warnings: Vec::new(),
            search_query: String::new(),
            counter: 0,
            status: "Chưa có profile — hãy tạo profile trước".to_string(),
        }
    }
}

#[allow(dead_code)]
impl CttApp {
    /// Initializes application state with zero profiles so the user creates one first.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the currently selected navigation tab.
    pub fn active_tab(&self) -> NavTab {
        self.active_tab
    }

    /// Switches the active navigation tab.
    pub fn set_active_tab(&mut self, tab: NavTab) {
        self.active_tab = tab;
    }

    /// Returns `true` when at least one profile exists and is active.
    pub fn has_active_profile(&self) -> bool {
        !self.domain_profiles.is_empty()
    }

    /// Returns all available UI profile views.
    pub fn profiles(&self) -> &[Profile] {
        &self.ui_profiles
    }

    /// Returns the ordered display names of all profiles.
    pub fn profile_names(&self) -> Vec<String> {
        self.domain_profiles
            .iter()
            .map(|p| p.name.clone())
            .collect()
    }

    /// Returns the index of the currently active profile.
    pub fn active_profile_index(&self) -> usize {
        self.active_profile_index
    }

    /// Returns a UI projection of the currently active profile, or `None` if no profile exists.
    pub fn active_profile(&self) -> Option<&Profile> {
        self.ui_profiles.get(self.active_profile_index)
    }

    /// Returns a reference to the active domain [`DomainProfile`], or `None` if no profile exists.
    pub fn active_domain_profile(&self) -> Option<&DomainProfile> {
        self.domain_profiles.get(self.active_profile_index)
    }

    /// Returns the active profile name or a prompt label when no profile has been created yet.
    pub fn active_profile_name(&self) -> &str {
        self.active_domain_profile()
            .map(|p| p.name.as_str())
            .unwrap_or("Chưa có profile")
    }

    /// Selects a profile by index, releasing held outputs and swapping the compiled lookup table.
    pub fn select_profile(&mut self, index: usize) -> bool {
        if index >= self.domain_profiles.len() {
            return false;
        }
        self.active_profile_index = index;
        self.validation_error.clear();
        self.validation_warnings.clear();
        self.sync_active_profile_to_engine();
        self.status = format!("Selected {}", self.active_profile_name());
        true
    }

    /// Creates and activates a new empty profile, returning its index.
    pub fn create_profile(&mut self) -> usize {
        let name = format!("Profile {}", self.next_profile_number);
        self.next_profile_number += 1;

        let domain = DomainProfile::new(self.next_profile_id, name.clone());
        self.next_profile_id += 1;

        let ui = Profile::from_domain(&domain);
        self.domain_profiles.push(domain);
        self.ui_profiles.push(ui);
        self.active_profile_index = self.domain_profiles.len() - 1;
        self.validation_error.clear();
        self.validation_warnings.clear();
        self.sync_active_profile_to_engine();
        self.status = format!("Created {name}");
        self.active_profile_index
    }

    /// Deletes the active profile, returning `false` if no profiles exist.
    pub fn delete_active_profile(&mut self) -> bool {
        if self.domain_profiles.is_empty() {
            return false;
        }

        self.domain_profiles.remove(self.active_profile_index);
        self.ui_profiles.remove(self.active_profile_index);
        self.validation_error.clear();
        self.validation_warnings.clear();

        if self.domain_profiles.is_empty() {
            self.active_profile_index = 0;
            if self.engine.status() == EngineStatus::Active {
                let _ = self.engine.set_enabled(false, &mut self.synthesizer);
            }
            self.sync_active_profile_to_engine();
            self.status = "Đã xoá profile — hiện chưa có profile nào".to_string();
            return true;
        }

        if self.active_profile_index >= self.domain_profiles.len() {
            self.active_profile_index = self.domain_profiles.len() - 1;
        }
        self.sync_active_profile_to_engine();
        self.status = format!("Active profile: {}", self.active_profile_name());
        true
    }

    /// Returns the bindings belonging to the active profile, or an empty slice if no profile exists.
    pub fn active_bindings(&self) -> &[BindingEntry] {
        self.active_profile()
            .map(|p| p.bindings.as_slice())
            .unwrap_or(&[])
    }

    /// Returns detailed binding rows (including source keyboard key) for the Binding tab.
    pub fn domain_rule_views(&self) -> Vec<DomainRuleView> {
        let Some(profile) = self.active_domain_profile() else {
            return Vec::new();
        };

        profile
            .rules
            .iter()
            .map(|rule| DomainRuleView {
                id: rule.id,
                source_key: rule.source,
                source_label: rule.source.display_name(),
                target_label: rule.target.display_name(),
                label: rule.label.clone(),
                enabled: rule.enabled,
            })
            .collect()
    }

    /// Formats the binding table footer summary for the active profile.
    pub fn binding_summary(&self) -> String {
        if !self.has_active_profile() {
            return "Chưa có profile · Hãy tạo profile trước để bắt đầu bind phím".to_string();
        }
        format!(
            "{} binding · {}",
            self.active_bindings().len(),
            self.active_profile_name()
        )
    }

    /// Adds or updates a binding on the active profile after validating conflict and safety rules.
    ///
    /// Fails with [`MappingError::NoActiveProfile`] if no profile has been created yet.
    pub fn bind_key(
        &mut self,
        source: impl Into<InputSource>,
        target: MappingTarget,
        label: impl Into<String>,
    ) -> Result<(), Vec<MappingError>> {
        let source = source.into();
        if !self.has_active_profile() {
            let msg = "Vui lòng tạo profile trước khi bind phím.".to_string();
            self.validation_error = msg.clone();
            self.status = msg;
            return Err(vec![MappingError::NoActiveProfile]);
        }

        let label_str = label.into();
        let effective_label = if label_str.trim().is_empty() {
            format!("{} → {}", source.display_name(), target.display_name())
        } else {
            label_str
        };

        let idx = self.active_profile_index;
        let mut candidate = self.domain_profiles[idx].clone();
        candidate.set_or_add_binding(source, target, effective_label);

        let report = validate_profile_rules(&candidate.rules, self.engine.emergency_stop_hotkey());
        if !report.is_valid() {
            self.validation_error = report
                .errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(report.errors);
        }

        self.validation_error.clear();
        self.validation_warnings = report
            .warnings
            .iter()
            .map(MappingWarning::to_string)
            .collect();
        self.domain_profiles[idx] = candidate;
        self.ui_profiles[idx] = Profile::from_domain(&self.domain_profiles[idx]);
        self.sync_active_profile_to_engine();
        self.status = format!("Bound {}", source.display_name());
        Ok(())
    }

    /// Removes the binding for `source` from the active profile.
    pub fn remove_binding(&mut self, source: impl Into<InputSource>) -> bool {
        let source = source.into();
        if !self.has_active_profile() {
            return false;
        }
        let idx = self.active_profile_index;
        if !self.domain_profiles[idx].remove_binding_by_source(source) {
            return false;
        }
        self.ui_profiles[idx] = Profile::from_domain(&self.domain_profiles[idx]);
        self.validation_error.clear();
        self.sync_active_profile_to_engine();
        self.status = format!("Removed binding for {}", source.display_name());
        true
    }

    /// Applies a new binding selected via UI dropdown/toggle indices.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_binding_selection(
        &mut self,
        source_idx: usize,
        target_kind_idx: usize,
        target_key_idx: usize,
        ctrl: bool,
        shift: bool,
        alt: bool,
        meta: bool,
        mouse_btn_idx: usize,
        wheel_idx: usize,
        label: &str,
    ) -> Result<(), String> {
        if !self.has_active_profile() {
            let msg = "Vui lòng tạo profile trước khi bind phím.".to_string();
            self.validation_error = msg.clone();
            self.status = msg.clone();
            return Err(msg);
        }

        let source_catalog = InputSource::all_standard();
        let Some(&source) = source_catalog.get(source_idx) else {
            let msg = "Invalid source input selection".to_string();
            self.validation_error = msg.clone();
            return Err(msg);
        };

        let target = match target_kind_idx {
            0 => {
                let Some(&key) = KeyboardKey::ALL_STANDARD.get(target_key_idx) else {
                    let msg = "Invalid target keyboard key selection".to_string();
                    self.validation_error = msg.clone();
                    return Err(msg);
                };
                MappingTarget::Keyboard(key)
            }
            1 => {
                let Some(&key) = KeyboardKey::ALL_STANDARD.get(target_key_idx) else {
                    let msg = "Invalid combo keyboard key selection".to_string();
                    self.validation_error = msg.clone();
                    return Err(msg);
                };
                let mut modifiers = ModifierMask::NONE;
                if ctrl {
                    modifiers = modifiers.with(ModifierMask::CTRL);
                }
                if shift {
                    modifiers = modifiers.with(ModifierMask::SHIFT);
                }
                if alt {
                    modifiers = modifiers.with(ModifierMask::ALT);
                }
                if meta {
                    modifiers = modifiers.with(ModifierMask::META);
                }
                MappingTarget::KeyCombo { modifiers, key }
            }
            2 => {
                let Some(&btn) = MouseButton::ALL.get(mouse_btn_idx) else {
                    let msg = "Invalid target mouse button selection".to_string();
                    self.validation_error = msg.clone();
                    return Err(msg);
                };
                MappingTarget::MouseButton(btn)
            }
            3 => {
                let Some(&wheel) = MouseWheel::ALL.get(wheel_idx) else {
                    let msg = "Invalid target mouse wheel selection".to_string();
                    self.validation_error = msg.clone();
                    return Err(msg);
                };
                MappingTarget::MouseWheel(wheel)
            }
            _ => MappingTarget::Disabled,
        };

        self.bind_key(source, target, label).map_err(|errors| {
            errors
                .into_iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })
    }

    /// Returns a read-only reference to the runtime [`RemapEngine`].
    pub fn engine(&self) -> &RemapEngine {
        &self.engine
    }

    /// Returns a mutable reference to the runtime [`RemapEngine`].
    pub fn engine_mut(&mut self) -> &mut RemapEngine {
        &mut self.engine
    }

    /// Enables or pauses the remapping engine safely.
    pub fn set_mapping_enabled(&mut self, enabled: bool) {
        if enabled && !self.has_active_profile() {
            self.status = "Vui lòng tạo profile trước khi kích hoạt".to_string();
            return;
        }
        if let Err(err) = self.engine.set_enabled(enabled, &mut self.synthesizer) {
            self.status = format!("Engine error: {err}");
            return;
        }
        self.status = if enabled {
            format!("Mapping Active ({})", self.active_profile_name())
        } else {
            "Mapping Paused".to_string()
        };
    }

    /// Immediately triggers Emergency Stop, releasing all active holds and disabling mapping.
    pub fn trigger_emergency_stop(&mut self) {
        if let Err(err) = self.engine.emergency_stop_all(&mut self.synthesizer) {
            self.status = format!("Emergency Stop error: {err}");
            return;
        }
        self.status = format!(
            "Emergency Stopped ({})",
            self.config.emergency_stop.display_name()
        );
    }

    /// Returns `true` when key remapping is currently active.
    pub fn is_mapping_enabled(&self) -> bool {
        self.engine.status() == EngineStatus::Active
    }

    /// Returns the current engine status label.
    pub fn engine_status(&self) -> EngineStatus {
        self.engine.status()
    }

    /// Returns the configured emergency stop hotkey label.
    pub fn emergency_stop_label(&self) -> String {
        self.config.emergency_stop.display_name()
    }

    /// Returns the latest validation error string (empty if valid).
    pub fn validation_error(&self) -> &str {
        &self.validation_error
    }

    /// Returns the latest non-blocking validation warnings.
    pub fn validation_warnings(&self) -> &[String] {
        &self.validation_warnings
    }

    /// Returns the current demo counter value.
    pub fn counter(&self) -> i32 {
        self.counter
    }

    /// Increments and returns the demo counter value.
    pub fn increment(&mut self) -> i32 {
        self.counter += 1;
        self.counter
    }

    /// Updates the search query string.
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    /// Returns the current search query string.
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Returns the current status text.
    pub fn status(&self) -> &str {
        &self.status
    }

    /// Sets the current status text.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    fn sync_active_profile_to_engine(&mut self) {
        let table = self
            .active_domain_profile()
            .map(DomainProfile::compile_table)
            .unwrap_or_default();
        if let Err(err) = self.engine.switch_table(table, &mut self.synthesizer) {
            self.status = format!("Engine sync error: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_initial_state_requires_creating_profile_before_binding() {
        let mut app = CttApp::new();
        assert_eq!(app.active_tab(), NavTab::Overview);
        assert!(!app.has_active_profile());
        assert!(app.profiles().is_empty());
        assert!(app.profile_names().is_empty());
        assert!(app.active_profile().is_none());
        assert!(app.active_domain_profile().is_none());
        assert!(app.active_bindings().is_empty());
        assert_eq!(app.active_profile_name(), "Chưa có profile");

        // Binding before creating a profile must be rejected
        let err = app
            .bind_key(
                KeyboardKey::Space,
                MappingTarget::MouseButton(MouseButton::Left),
                "Should fail",
            )
            .expect_err("Binding without profile must return MappingError::NoActiveProfile");
        assert_eq!(err, vec![MappingError::NoActiveProfile]);
        assert!(!app.validation_error().is_empty());

        // UI index helper also rejects when no profile exists
        assert!(app
            .apply_binding_selection(0, 0, 1, false, false, false, false, 0, 0, "Test")
            .is_err());

        // Cannot activate mapping without a profile
        app.set_mapping_enabled(true);
        assert!(!app.is_mapping_enabled());
    }

    #[test]
    fn test_create_and_select_empty_profiles() {
        let mut app = CttApp::new();
        let idx1 = app.create_profile();
        assert_eq!(idx1, 0);
        assert!(app.has_active_profile());
        assert_eq!(app.active_profile_name(), "Profile 1");
        assert!(app.active_bindings().is_empty());
        assert_eq!(app.profile_names(), vec!["Profile 1"]);

        let idx2 = app.create_profile();
        assert_eq!(idx2, 1);
        assert_eq!(app.active_profile_name(), "Profile 2");
        assert_eq!(app.profile_names(), vec!["Profile 1", "Profile 2"]);

        assert!(app.select_profile(0));
        assert_eq!(app.active_profile_index(), 0);
        assert_eq!(app.active_profile_name(), "Profile 1");

        assert!(!app.select_profile(99));
        assert_eq!(app.active_profile_index(), 0);
    }

    #[test]
    fn test_delete_profiles_returns_to_empty_gated_state() {
        let mut app = CttApp::new();

        // Deleting when no profile exists returns false
        assert!(!app.delete_active_profile());
        assert!(!app.has_active_profile());

        app.create_profile();
        app.create_profile();
        assert_eq!(app.profiles().len(), 2);
        assert_eq!(app.active_profile_index(), 1);

        assert!(app.delete_active_profile());
        assert_eq!(app.profiles().len(), 1);
        assert_eq!(app.active_profile_index(), 0);
        assert_eq!(app.active_profile_name(), "Profile 1");

        // Deleting the last remaining profile leaves zero profiles and blocks binding again
        assert!(app.delete_active_profile());
        assert!(!app.has_active_profile());
        assert!(app.profiles().is_empty());
        assert!(app.active_bindings().is_empty());
        assert_eq!(
            app.bind_key(
                KeyboardKey::KeyQ,
                MappingTarget::Keyboard(KeyboardKey::KeyE),
                "Should fail"
            ),
            Err(vec![MappingError::NoActiveProfile])
        );
    }

    #[test]
    fn test_nav_tab_switching() {
        let mut app = CttApp::new();
        assert_eq!(NavTab::from_index(0), NavTab::Overview);
        assert_eq!(NavTab::from_index(1), NavTab::Macros);
        assert_eq!(NavTab::from_index(2), NavTab::Binding);
        assert_eq!(NavTab::from_index(42), NavTab::Overview);

        app.set_active_tab(NavTab::Macros);
        assert_eq!(app.active_tab(), NavTab::Macros);
        assert_eq!(app.active_tab().as_index(), 1);

        app.set_active_tab(NavTab::Binding);
        assert_eq!(app.active_tab(), NavTab::Binding);
        assert_eq!(app.active_tab().as_index(), 2);
    }

    #[test]
    fn test_counter_increment() {
        let mut app = CttApp::new();
        assert_eq!(app.counter(), 0);
        assert_eq!(app.increment(), 1);
        assert_eq!(app.increment(), 2);
    }

    #[test]
    fn test_search_query() {
        let mut app = CttApp::new();
        assert_eq!(app.search_query(), "");
        app.set_search_query("macro_1".to_string());
        assert_eq!(app.search_query(), "macro_1");
    }

    #[test]
    fn test_bind_keyboard_to_mouse_and_emergency_stop() {
        let mut app = CttApp::new();
        app.create_profile();

        app.bind_key(
            KeyboardKey::Space,
            MappingTarget::MouseButton(MouseButton::Left),
            "Jump Click",
        )
        .expect("Valid key-to-mouse binding should succeed once a profile exists");

        assert!(app.domain_rule_views().iter().any(|r| r.source_key
            == InputSource::Keyboard(KeyboardKey::Space)
            && r.target_label == "Mouse Left"));

        app.bind_key(
            MouseButton::XButton1,
            MappingTarget::Keyboard(KeyboardKey::KeyC),
            "Side1 -> C",
        )
        .expect("Mouse button source binding should succeed");

        assert!(app.domain_rule_views().iter().any(|r| r.source_key
            == InputSource::MouseButton(MouseButton::XButton1)
            && r.target_label == "C"));

        app.set_mapping_enabled(true);
        assert!(app.is_mapping_enabled());

        app.trigger_emergency_stop();
        assert!(!app.is_mapping_enabled());
        assert_eq!(app.engine_status(), EngineStatus::EmergencyStopped);
    }
}
