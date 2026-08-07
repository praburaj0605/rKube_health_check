//! Startup probe state machine (FR-004).

use serde::{Deserialize, Serialize};

/// Startup probe lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum StartupState {
    /// Process just started.
    Initializing,
    /// Loading configuration.
    LoadingConfiguration,
    /// Connecting to dependencies.
    ConnectingDependencies,
    /// Startup complete; ready for traffic.
    Ready,
    /// Startup failed permanently.
    Failed,
}

impl StartupState {
    /// Advance to the next logical startup state.
    pub fn advance(self) -> Self {
        match self {
            StartupState::Initializing => StartupState::LoadingConfiguration,
            StartupState::LoadingConfiguration => StartupState::ConnectingDependencies,
            StartupState::ConnectingDependencies => StartupState::Ready,
            StartupState::Ready => StartupState::Ready,
            StartupState::Failed => StartupState::Failed,
        }
    }

    /// Returns `true` when the startup probe should return HTTP 200.
    pub fn is_ready(self) -> bool {
        matches!(self, StartupState::Ready)
    }

    /// Returns `true` when startup has permanently failed.
    pub fn is_failed(self) -> bool {
        matches!(self, StartupState::Failed)
    }
}

impl std::fmt::Display for StartupState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartupState::Initializing => write!(f, "Initializing"),
            StartupState::LoadingConfiguration => write!(f, "LoadingConfiguration"),
            StartupState::ConnectingDependencies => write!(f, "ConnectingDependencies"),
            StartupState::Ready => write!(f, "Ready"),
            StartupState::Failed => write!(f, "Failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_to_ready() {
        let mut s = StartupState::Initializing;
        s = s.advance();
        assert_eq!(s, StartupState::LoadingConfiguration);
        s = s.advance();
        assert_eq!(s, StartupState::ConnectingDependencies);
        s = s.advance();
        assert_eq!(s, StartupState::Ready);
        assert!(s.is_ready());
        assert_eq!(s.advance(), StartupState::Ready);
    }
}
