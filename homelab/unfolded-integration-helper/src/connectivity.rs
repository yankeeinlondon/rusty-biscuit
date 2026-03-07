use std::collections::HashMap;

/// Integration-level device connectivity state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectivityState {
    #[default]
    Unknown,
    Connected,
    Disconnected,
    Error,
}

impl ConnectivityState {
    #[must_use]
    pub fn as_uc_state(self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Connected => "CONNECTED",
            Self::Disconnected => "DISCONNECTED",
            Self::Error => "ERROR",
        }
    }

    #[must_use]
    pub fn as_uc_label(self) -> &'static str {
        self.as_uc_state()
    }
}

/// Tracks per-device connectivity and computes an overall integration state.
#[derive(Debug, Clone, Default)]
pub struct ConnectivityTracker {
    states: HashMap<String, ConnectivityState>,
}

impl ConnectivityTracker {
    #[must_use]
    pub fn new(device_names: impl IntoIterator<Item = String>) -> Self {
        let mut states = HashMap::new();
        for name in device_names {
            states.insert(name, ConnectivityState::Unknown);
        }
        Self { states }
    }

    pub fn set(
        &mut self,
        device_name: impl Into<String>,
        state: ConnectivityState,
    ) -> Option<ConnectivityState> {
        self.states.insert(device_name.into(), state)
    }

    #[must_use]
    pub fn get(&self, device_name: &str) -> ConnectivityState {
        self.states
            .get(device_name)
            .copied()
            .unwrap_or(ConnectivityState::Unknown)
    }

    #[must_use]
    pub fn overall(&self) -> ConnectivityState {
        if self.states.is_empty() {
            return ConnectivityState::Disconnected;
        }

        let mut has_unknown = false;
        let mut has_error = false;

        for state in self.states.values().copied() {
            match state {
                ConnectivityState::Connected => return ConnectivityState::Connected,
                ConnectivityState::Error => has_error = true,
                ConnectivityState::Unknown => has_unknown = true,
                ConnectivityState::Disconnected => {}
            }
        }

        if has_error {
            ConnectivityState::Error
        } else if has_unknown {
            ConnectivityState::Unknown
        } else {
            ConnectivityState::Disconnected
        }
    }

    pub fn mark_connected(&mut self, device_name: &str) -> bool {
        self.set(device_name.to_string(), ConnectivityState::Connected)
            != Some(ConnectivityState::Connected)
    }

    pub fn mark_disconnected(&mut self, device_name: &str) -> bool {
        self.set(device_name.to_string(), ConnectivityState::Disconnected)
            != Some(ConnectivityState::Disconnected)
    }

    #[must_use]
    pub fn aggregate_state(&self) -> ConnectivityState {
        self.overall()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overall_prefers_connected_then_error_then_unknown() {
        let mut tracker = ConnectivityTracker::new(["living".to_string(), "office".to_string()]);
        assert_eq!(tracker.overall(), ConnectivityState::Unknown);

        tracker.set("living", ConnectivityState::Disconnected);
        tracker.set("office", ConnectivityState::Error);
        assert_eq!(tracker.overall(), ConnectivityState::Error);

        tracker.set("office", ConnectivityState::Connected);
        assert_eq!(tracker.overall(), ConnectivityState::Connected);
    }
}
