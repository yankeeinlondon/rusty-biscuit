//! Failure injection at named protocol steps.

/// A protocol step at which a crash can be injected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Point {
    /// Lock held, nothing written.
    BeforeStaging,
    /// First new artifact staged, rest not.
    DuringStaging,
    /// All new bytes and backups staged; journal not yet written.
    StagedNoJournal,
    /// Journal `prepared` durable; no fixed path touched (A) / pointer not
    /// replaced (B).
    BeforeSelection,
    /// A: after this many fixed artifacts were replaced (manifest untouched).
    /// B: after this many fixed paths were mirrored.
    MidReplacement(usize),
    /// A: all artifacts replaced, manifest not yet replaced.
    BeforeManifest,
    /// A: manifest replaced (committed); journal not yet removed.
    /// B: pointer replaced; mirror not started.
    AfterSelection,
    /// A: journal removed; staging directory not yet removed.
    BeforeCleanup,
}

impl Point {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "BeforeStaging" => Self::BeforeStaging,
            "DuringStaging" => Self::DuringStaging,
            "StagedNoJournal" => Self::StagedNoJournal,
            "BeforeSelection" => Self::BeforeSelection,
            "BeforeManifest" => Self::BeforeManifest,
            "AfterSelection" => Self::AfterSelection,
            "BeforeCleanup" => Self::BeforeCleanup,
            other => Self::MidReplacement(other.strip_prefix("MidReplacement")?.parse().ok()?),
        })
    }

    pub fn label(self) -> String {
        match self {
            Self::MidReplacement(n) => format!("MidReplacement{n}"),
            other => format!("{other:?}"),
        }
    }
}

/// What a fired fault does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Return [`crate::SpikeError::Crash`]; destructors still run.
    Error,
    /// `std::process::abort()`: no destructors, no cleanup, lock released by the OS.
    Abort,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Faults {
    pub at: Option<(Point, Mode)>,
}

impl Faults {
    pub fn none() -> Self {
        Self { at: None }
    }

    pub fn error_at(point: Point) -> Self {
        Self { at: Some((point, Mode::Error)) }
    }

    pub fn abort_at(point: Point) -> Self {
        Self { at: Some((point, Mode::Abort)) }
    }

    pub(crate) fn check(&self, point: Point) -> Result<(), crate::SpikeError> {
        match self.at {
            Some((p, Mode::Error)) if p == point => Err(crate::SpikeError::Crash(point)),
            Some((p, Mode::Abort)) if p == point => std::process::abort(),
            _ => Ok(()),
        }
    }
}
