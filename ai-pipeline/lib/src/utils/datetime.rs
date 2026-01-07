use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// An **Epoch** timestamp measured in seconds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Epoch(u32);

impl Epoch {
    /// Returns the current time as an Epoch timestamp.
    pub fn now() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch");
        Self(duration.as_secs() as u32)
    }
}
