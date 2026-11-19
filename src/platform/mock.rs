//! In-memory mock [`InputSynthesizer`] for deterministic unit and integration tests.

use crate::platform::{InputSynthesizer, PlatformError, SyntheticOutput};

/// Records all synthesized keyboard/mouse events in memory without touching host OS input.
#[derive(Debug, Default)]
pub struct MockInputSynthesizer {
    events: Vec<SyntheticOutput>,
    fail_with: Option<PlatformError>,
}

impl MockInputSynthesizer {
    /// Creates an empty mock synthesizer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the next [`InputSynthesizer::synthesize`] call to return `err`.
    pub fn inject_next_error(&mut self, err: PlatformError) {
        self.fail_with = Some(err);
    }

    /// Returns all synthesized events recorded so far.
    pub fn events(&self) -> &[SyntheticOutput] {
        &self.events
    }

    /// Clears all recorded events and returns them.
    pub fn drain_events(&mut self) -> Vec<SyntheticOutput> {
        std::mem::take(&mut self.events)
    }
}

impl InputSynthesizer for MockInputSynthesizer {
    fn synthesize(&mut self, output: SyntheticOutput) -> Result<(), PlatformError> {
        if let Some(err) = self.fail_with.take() {
            return Err(err);
        }
        self.events.push(output);
        Ok(())
    }
}
