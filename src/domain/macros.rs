//! Placeholder macro identifiers and playback modes for upcoming macro integration.

/// Stable identifier referencing a saved macro definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacroId(pub u32);

/// Playback execution behavior for future macro triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MacroPlaybackMode {
    RunOnce,
    RepeatCount(u32),
    ToggleLoop,
    HoldLoop,
}
