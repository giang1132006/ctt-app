//! OS-level input hook and event synthesis abstractions.
//!
//! All unsafe OS FFI and platform key code translation live inside submodules
//! of [`crate::platform`], while [`crate::engine`] interacts solely through
//! [`InputSynthesizer`] and [`InputInterceptor`].

pub mod mock;
pub mod windows;

use std::fmt;

use crate::domain::keys::{KeyState, KeyboardKey, MouseButton, MouseWheel};

/// Tag stamped onto every event synthesized by `ctt-app` (`"CTT_SYNT"` in ASCII hex)
/// so low-level OS hooks ignore self-generated events and prevent infinite loops.
pub const CTT_SYNTHETIC_TAG: usize = 0x4354_545F_5359_4E54;

/// Normalized physical keyboard event passed from a platform hook to [`crate::engine::remapper::RemapEngine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawKeyEvent {
    pub key: KeyboardKey,
    pub state: KeyState,
    pub is_injected: bool,
}

impl RawKeyEvent {
    /// Creates a physical (non-injected) raw key event.
    pub const fn physical(key: KeyboardKey, state: KeyState) -> Self {
        Self {
            key,
            state,
            is_injected: false,
        }
    }

    /// Creates an injected (self-synthesized or external software) raw key event.
    pub const fn injected(key: KeyboardKey, state: KeyState) -> Self {
        Self {
            key,
            state,
            is_injected: true,
        }
    }
}

/// Normalized physical mouse button event passed from a platform hook to [`crate::engine::remapper::RemapEngine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawMouseButtonEvent {
    pub button: MouseButton,
    pub state: KeyState,
    pub is_injected: bool,
}

impl RawMouseButtonEvent {
    /// Creates a physical (non-injected) raw mouse button event.
    pub const fn physical(button: MouseButton, state: KeyState) -> Self {
        Self {
            button,
            state,
            is_injected: false,
        }
    }

    /// Creates an injected (self-synthesized or external software) raw mouse button event.
    pub const fn injected(button: MouseButton, state: KeyState) -> Self {
        Self {
            button,
            state,
            is_injected: true,
        }
    }
}

/// Decision returned by the hook evaluator instructing the OS hook whether to forward or swallow input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookDecision {
    PassThrough,
    Suppress,
}

/// Platform-neutral synthesized keyboard or mouse output event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticOutput {
    Key {
        key: KeyboardKey,
        state: KeyState,
    },
    MouseButton {
        button: MouseButton,
        state: KeyState,
    },
    MouseWheel {
        direction: MouseWheel,
        delta: i32,
    },
}

/// Typed errors returned by platform input interception and synthesis operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    HookRegistrationFailed(String),
    SynthesisFailed(String),
    UnsupportedPlatform,
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HookRegistrationFailed(msg) => write!(f, "Hook registration failed: {msg}"),
            Self::SynthesisFailed(msg) => write!(f, "Input synthesis failed: {msg}"),
            Self::UnsupportedPlatform => {
                write!(f, "Live OS input backend is not enabled on this platform")
            }
        }
    }
}

impl std::error::Error for PlatformError {}

/// Abstraction for emitting synthesized keyboard and mouse events to the OS (or a test double).
pub trait InputSynthesizer: Send {
    fn synthesize(&mut self, output: SyntheticOutput) -> Result<(), PlatformError>;
}

/// Abstraction for managing an OS-level low-level input hook lifecycle.
pub trait InputInterceptor: Send {
    fn is_active(&self) -> bool;
    fn stop(&mut self) -> Result<(), PlatformError>;
}
