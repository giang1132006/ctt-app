//! Pure domain layer for key binding rules, validation, profiles, and configuration.
//!
//! This module is intentionally decoupled from UI (`slint`), OS hooks (`platform`),
//! and persistence (`storage`) so all business invariants remain unit-testable.

pub mod config;
pub mod keys;
pub mod macro_io;
pub mod macros;
pub mod mapping;
pub mod profile;
pub mod validation;
