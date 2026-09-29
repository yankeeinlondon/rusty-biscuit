//! Steering target identities.
//!
//! A steering target ID is opaque and exact: callers compare whole IDs and
//! never resolve prefixes. A PID alone is never an identity, because the OS
//! reuses PIDs; native IDs pair it with a process-start marker, and managed IDs
//! are per-execution values bound to the wrapper incarnation and the provider
//! conversation generation ([`ManagedTarget`]).

use std::fmt;
use std::str::FromStr;

use serde::{Serialize, Serializer};

use crate::provider_id::{PROVIDERS_DISPLAY_ORDER, Provider};

/// Why a target or identifier string was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    #[error("`{0}` is not a canonical lowercase hyphenated 128-bit identifier")]
    MalformedUuid(String),
    #[error("target ID must start with `managed:` or `native:`")]
    UnknownKind,
    #[error("native target ID needs provider, PID, process start, and conversation parts")]
    MissingPart,
    #[error("unknown provider `{0}` in target ID")]
    UnknownProvider(String),
    #[error("`{0}` is not a process ID")]
    InvalidPid(String),
    #[error("process-start marker must be nonempty and contain no `:`")]
    InvalidProcessStart,
}

fn format_u128(value: u128, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let hex = format!("{value:032x}");
    write!(f, "{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

fn parse_u128(text: &str) -> Result<u128, IdentityError> {
    let malformed = || IdentityError::MalformedUuid(text.to_string());
    let groups: Vec<&str> = text.split('-').collect();
    let lengths: Vec<usize> = groups.iter().map(|group| group.len()).collect();
    if lengths != [8, 4, 4, 4, 12]
        || !text.chars().all(|c| c == '-' || c.is_ascii_digit() || ('a'..='f').contains(&c))
    {
        return Err(malformed());
    }
    u128::from_str_radix(&groups.concat(), 16).map_err(|_| malformed())
}

macro_rules! uuid_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u128);

        impl $name {
            /// Wraps a 128-bit value (for example a random v4 UUID).
            pub const fn from_u128(value: u128) -> Self {
                Self(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                format_u128(self.0, f)
            }
        }

        impl FromStr for $name {
            type Err = IdentityError;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                parse_u128(text).map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }
    };
}

uuid_id! {
    /// Per-execution managed identity, distinct from any resumable provider
    /// conversation ID.
    ExecutionId
}

uuid_id! {
    /// Identity of one steering request; correlation never reuses it.
    RequestId
}

/// Monotonic counter of provider conversations within one managed execution.
/// A conversation switch increments it and invalidates older targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ConversationGeneration(pub u64);

/// A process identity that survives PID reuse: the PID plus an OS-provided
/// start marker (for example the process start time in clock ticks).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ProcessStartIdentity {
    pub pid: u32,
    start: String,
}

impl ProcessStartIdentity {
    /// Pairs a PID with its start marker. The marker cannot contain `:`, which
    /// delimits target-ID parts.
    pub fn new(pid: u32, start: impl Into<String>) -> Result<Self, IdentityError> {
        let start = start.into();
        if start.is_empty() || start.contains(':') {
            return Err(IdentityError::InvalidProcessStart);
        }
        Ok(Self { pid, start })
    }

    /// The opaque process-start marker.
    pub fn start(&self) -> &str {
        &self.start
    }
}

/// The exact ID accepted by `claudine steer --session`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SteeringTargetId {
    /// An execution Claudine owns; routed to its controller.
    Managed { execution: ExecutionId },
    /// A natively discovered provider session.
    Native {
        provider: Provider,
        process: ProcessStartIdentity,
        /// Provider conversation ID; the final ID part, so it may contain `:`.
        conversation: String,
    },
}

impl fmt::Display for SteeringTargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Managed { execution } => write!(f, "managed:{execution}"),
            Self::Native { provider, process, conversation } => write!(
                f,
                "native:{}:{}:{}:{conversation}",
                provider.as_slug(),
                process.pid,
                process.start
            ),
        }
    }
}

impl FromStr for SteeringTargetId {
    type Err = IdentityError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if let Some(execution) = text.strip_prefix("managed:") {
            return Ok(Self::Managed { execution: execution.parse()? });
        }
        let rest = text.strip_prefix("native:").ok_or(IdentityError::UnknownKind)?;
        let mut parts = rest.splitn(4, ':');
        let mut next = || parts.next().filter(|part| !part.is_empty()).ok_or(IdentityError::MissingPart);
        let slug = next()?;
        let pid = next()?;
        let start = next()?;
        let conversation = next()?;
        let provider = PROVIDERS_DISPLAY_ORDER
            .iter()
            .copied()
            .find(|provider| provider.as_slug() == slug)
            .ok_or_else(|| IdentityError::UnknownProvider(slug.to_string()))?;
        let pid = pid
            .parse::<u32>()
            .ok()
            .filter(|_| pid.bytes().all(|b| b.is_ascii_digit()))
            .ok_or_else(|| IdentityError::InvalidPid(pid.to_string()))?;
        Ok(Self::Native {
            provider,
            process: ProcessStartIdentity::new(pid, start)?,
            conversation: conversation.to_string(),
        })
    }
}

impl Serialize for SteeringTargetId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// Everything a managed target was bound to when it was listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedTarget {
    pub execution: ExecutionId,
    pub wrapper: ProcessStartIdentity,
    pub generation: ConversationGeneration,
    /// Provider conversation ID, when the provider has reported one.
    pub conversation: Option<String>,
}

/// Why a previously observed target no longer names the live session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StaleTarget {
    #[error("the owning wrapper process changed")]
    WrapperChanged,
    #[error("the provider conversation was replaced")]
    ConversationReplaced,
}

impl ManagedTarget {
    /// The selectable ID for this target.
    pub fn id(&self) -> SteeringTargetId {
        SteeringTargetId::Managed { execution: self.execution }
    }

    /// Revalidates an observed target against the owner's current binding.
    /// A changed binding is rejected, never retargeted: the request must not
    /// reach a different conversation than the one the user selected.
    pub fn revalidate(&self, current: &ManagedTarget) -> Result<(), StaleTarget> {
        if self.execution != current.execution || self.wrapper != current.wrapper {
            return Err(StaleTarget::WrapperChanged);
        }
        if self.generation != current.generation || self.conversation != current.conversation {
            return Err(StaleTarget::ConversationReplaced);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW: u128 = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef;

    #[test]
    fn managed_ids_round_trip_exactly() {
        let id = SteeringTargetId::Managed { execution: ExecutionId::from_u128(RAW) };
        let text = id.to_string();
        assert_eq!(text, "managed:01234567-89ab-cdef-0123-456789abcdef");
        assert_eq!(text.parse::<SteeringTargetId>().unwrap(), id);
    }

    #[test]
    fn identifiers_reject_prefixes_uppercase_and_garbage() {
        for bad in [
            "01234567",
            "01234567-89ab-cdef-0123-456789abcdeF",
            "01234567-89ab-cdef-0123-456789abcdef0",
            "01234567-89ab-cdef-0123-456789abcdef ",
            "g1234567-89ab-cdef-0123-456789abcdef",
            "",
        ] {
            assert!(bad.parse::<ExecutionId>().is_err(), "{bad:?}");
        }
        assert_eq!("managed:01234567".parse::<SteeringTargetId>().unwrap_err(), IdentityError::MalformedUuid("01234567".into()));
        assert_eq!("session:x".parse::<SteeringTargetId>().unwrap_err(), IdentityError::UnknownKind);
    }

    #[test]
    fn native_ids_keep_colons_in_the_conversation_part() {
        let id = SteeringTargetId::Native {
            provider: Provider::Pi,
            process: ProcessStartIdentity::new(4242, "1711111111").unwrap(),
            conversation: "session:with:colons".into(),
        };
        let text = id.to_string();
        assert_eq!(text, "native:pi:4242:1711111111:session:with:colons");
        assert_eq!(text.parse::<SteeringTargetId>().unwrap(), id);
    }

    #[test]
    fn same_pid_with_a_new_start_marker_is_a_different_target() {
        let first: SteeringTargetId = "native:codex:77:100:thread".parse().unwrap();
        let reused: SteeringTargetId = "native:codex:77:200:thread".parse().unwrap();
        assert_ne!(first, reused);
    }

    #[test]
    fn native_ids_reject_missing_or_invalid_parts() {
        assert_eq!("native:pi:1:2".parse::<SteeringTargetId>().unwrap_err(), IdentityError::MissingPart);
        assert_eq!("native:pi:1::conv".parse::<SteeringTargetId>().unwrap_err(), IdentityError::MissingPart);
        assert_eq!("native:nope:1:2:c".parse::<SteeringTargetId>().unwrap_err(), IdentityError::UnknownProvider("nope".into()));
        assert_eq!("native:pi:+1:2:c".parse::<SteeringTargetId>().unwrap_err(), IdentityError::InvalidPid("+1".into()));
        assert_eq!("native:pi:-1:2:c".parse::<SteeringTargetId>().unwrap_err(), IdentityError::InvalidPid("-1".into()));
        assert_eq!(ProcessStartIdentity::new(1, "a:b").unwrap_err(), IdentityError::InvalidProcessStart);
        assert_eq!(ProcessStartIdentity::new(1, "").unwrap_err(), IdentityError::InvalidProcessStart);
    }

    #[test]
    fn conversation_switch_invalidates_a_managed_target() {
        let observed = ManagedTarget {
            execution: ExecutionId::from_u128(RAW),
            wrapper: ProcessStartIdentity::new(10, "5").unwrap(),
            generation: ConversationGeneration(1),
            conversation: Some("a".into()),
        };
        assert_eq!(observed.revalidate(&observed.clone()), Ok(()));
        let switched = ManagedTarget { generation: ConversationGeneration(2), ..observed.clone() };
        assert_eq!(observed.revalidate(&switched), Err(StaleTarget::ConversationReplaced));
        let renamed = ManagedTarget { conversation: Some("b".into()), ..observed.clone() };
        assert_eq!(observed.revalidate(&renamed), Err(StaleTarget::ConversationReplaced));
        let restarted = ManagedTarget { wrapper: ProcessStartIdentity::new(10, "6").unwrap(), ..observed.clone() };
        assert_eq!(observed.revalidate(&restarted), Err(StaleTarget::WrapperChanged));
    }

    #[test]
    fn ids_serialize_as_their_exact_text() {
        let id: SteeringTargetId = "native:pi:1:2:c".parse().unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"native:pi:1:2:c\"");
    }
}
