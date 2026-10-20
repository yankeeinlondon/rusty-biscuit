//! Per-call options for a raw send, and the state behind a
//! [`Call`](crate::Call)'s per-call settings.

use crate::credentials::Credential;
use crate::deadline::Deadline;

/// Options for one send: its credential source and an optional shared
/// deadline.
///
/// By default a send uses normal credential precedence: client credential,
/// then the configured environment variables, then anonymous. A per-call
/// [`credential`](Self::credential) overrides those, and
/// [`anonymous`](Self::anonymous) bypasses every credential source; the two
/// are mutually exclusive, and the last one set wins.
#[derive(Debug, Clone, Default)]
pub struct CallOptions {
    pub(crate) auth: CallAuth,
    pub(crate) deadline: Option<Deadline>,
}

/// The credential source a call asks for.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum CallAuth {
    /// Normal precedence.
    #[default]
    Inherit,
    /// This credential, overriding client and environment credentials.
    PerCall(Credential),
    /// No credential from any source.
    Anonymous,
}

impl CallOptions {
    /// Options with normal credential precedence and no deadline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Options that send no credential from any source, and remove
    /// authentication headers, query values, and cookies from the request.
    pub fn anonymous() -> Self {
        Self {
            auth: CallAuth::Anonymous,
            deadline: None,
        }
    }

    /// Sends `credential` for this call, replacing anonymous mode.
    #[must_use]
    pub fn credential(mut self, credential: Credential) -> Self {
        self.auth = CallAuth::PerCall(credential);
        self
    }

    /// Bounds this call by `deadline`, shared with its other holders.
    #[must_use]
    pub fn deadline(mut self, deadline: &Deadline) -> Self {
        self.deadline = Some(deadline.clone());
        self
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn the_default_inherits_credentials_and_has_no_deadline() {
        let options = CallOptions::new();
        assert_eq!(options.auth, CallAuth::Inherit);
        assert!(options.deadline.is_none());
    }

    #[test]
    fn a_credential_and_anonymous_mode_replace_each_other() {
        let token = Credential::access_token("t");
        let per_call = CallOptions::anonymous().credential(token.clone());
        assert_eq!(per_call.auth, CallAuth::PerCall(token));
        assert_eq!(CallOptions::anonymous().auth, CallAuth::Anonymous);
    }

    #[test]
    fn a_deadline_is_shared_not_copied() {
        let deadline = Deadline::after(Duration::from_secs(5));
        let options = CallOptions::anonymous().deadline(&deadline);
        assert_eq!(options.auth, CallAuth::Anonymous);
        let held = options.deadline.expect("deadline");
        assert_eq!(held.expires_at(), deadline.expires_at());
    }
}
