//! Command, status, and snapshot types bridging the UI controller and remapping engine.

use crate::domain::mapping::CompiledMappingTable;

/// Operational status of the key remapping runtime engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStatus {
    Idle,
    Active,
    Paused,
    EmergencyStopped,
}

impl EngineStatus {
    /// Human-readable label for status indicators.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Active => "Active",
            Self::Paused => "Paused",
            Self::EmergencyStopped => "Emergency Stopped",
        }
    }
}

/// Asynchronous commands dispatched from UI controls to the engine runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineCommand {
    SetEnabled(bool),
    SwitchProfileTable {
        profile_name: String,
        table: CompiledMappingTable,
    },
    EmergencyStop,
}

/// Lightweight immutable status snapshot read by the UI thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineSnapshot {
    pub status: EngineStatus,
    pub active_holds_count: usize,
    pub active_rule_count: usize,
}
