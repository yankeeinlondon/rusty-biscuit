//! How a finding was reached.

use std::fmt;

use serde::Serialize;

use crate::markdown::compose::expression::diagnostics::DiagnosticSource;
use crate::markdown::compose::expression::iteration::IterationPosition;

/// One step on the way to a finding.
///
/// Serializes tagged by `kind`: `included`, `iteration`, or `host`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TrailFrame {
    /// Reached through a transclusion at this authored position.
    Included {
        /// The including document, as a portable path or URL.
        file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        line: Option<usize>,
    },
    /// Inside a `::map` / `::filter` item.
    Iteration {
        /// The source reference spelling, such as `teams`.
        collection: String,
        /// The item's original position.
        position: IterationPosition,
    },
    /// A host construct, such as a Claudine task.
    Host {
        /// The host's noun for the construct (`task`, `group`, `sequence`).
        host: &'static str,
        /// The construct's display label, such as a task name.
        label: String,
        /// The authored property path, such as `sequence[1].prompt`.
        property: String,
        /// Where the construct is authored, when proven.
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<DiagnosticSource>,
    },
}

impl fmt::Display for TrailFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Included { file, line: Some(line) } => write!(f, "{file}:{line}"),
            Self::Included { file, line: None } => f.write_str(file),
            Self::Iteration { collection, position } => write!(f, "{collection}[{position}]"),
            Self::Host { host, label, property, .. } => write!(f, "{host} {label} ({property})"),
        }
    }
}

/// The frames that led to a finding, outermost first.
///
/// Renders on one line as `task metadata (sequence[1]) → _frontmatter.md`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Trail(Vec<TrailFrame>);

impl Trail {
    /// A trail of `frames`, outermost first.
    pub fn new(frames: Vec<TrailFrame>) -> Self {
        Self(frames)
    }

    /// A trail holding one host frame.
    pub fn host(host: &'static str, label: impl Into<String>, property: impl Into<String>) -> Self {
        Self(vec![TrailFrame::Host { host, label: label.into(), property: property.into(), source: None }])
    }

    /// This trail extended by one inner frame.
    #[must_use]
    pub fn pushed(mut self, frame: TrailFrame) -> Self {
        self.0.push(frame);
        self
    }

    #[allow(missing_docs)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(missing_docs)]
    pub fn frames(&self) -> &[TrailFrame] {
        &self.0
    }
}

impl fmt::Display for Trail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, frame) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(" → ")?;
            }
            frame.fmt(f)?;
        }
        Ok(())
    }
}
