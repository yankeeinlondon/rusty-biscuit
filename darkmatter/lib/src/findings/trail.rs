//! How a finding was reached.

use std::fmt;

use serde::Serialize;

use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::expression::diagnostics::DiagnosticSource;
use crate::markdown::compose::expression::iteration::{IterationFrame, IterationPath, IterationPosition};
use crate::markdown::reference::types::ReferenceTrailStep;

/// One step on the way to a finding.
///
/// Serializes tagged by `kind`: `included`, `iteration`, or `host`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TrailFrame {
    /// Reached through a transclusion at this authored position.
    Included {
        /// The including document; serializes as its path or URL, or `null`
        /// for input with no named origin.
        file: ComposeSource,
        /// The authored line of the including directive, when proven.
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

impl From<&ReferenceTrailStep> for TrailFrame {
    fn from(step: &ReferenceTrailStep) -> Self {
        Self::Included { file: step.file.clone(), line: step.line }
    }
}

impl From<&IterationFrame> for TrailFrame {
    fn from(frame: &IterationFrame) -> Self {
        Self::Iteration { collection: frame.collection.clone(), position: frame.position.clone() }
    }
}

impl fmt::Display for TrailFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Included { file, line: Some(line) } => write!(f, "{}:{line}", file.display()),
            Self::Included { file, line: None } => f.write_str(&file.display()),
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

    /// A trail of one included frame per step of `steps`, outermost first.
    pub fn included<'a>(steps: impl IntoIterator<Item = &'a ReferenceTrailStep>) -> Self {
        Self(steps.into_iter().map(TrailFrame::from).collect())
    }

    #[allow(missing_docs)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[allow(missing_docs)]
    pub fn frames(&self) -> &[TrailFrame] {
        &self.0
    }

    /// Whether a host pushed any frame. Only such a trail is published as
    /// `trail`; Darkmatter's own frames are published through the
    /// `iteration_path` and `included_via` projections.
    pub fn has_host_frame(&self) -> bool {
        self.0.iter().any(|frame| matches!(frame, TrailFrame::Host { .. }))
    }

    /// The iteration frames, outermost first, as the legacy path.
    pub fn iteration_path(&self) -> IterationPath {
        self.0.iter().fold(IterationPath::empty(), |path, frame| match frame {
            TrailFrame::Iteration { collection, position } => {
                path.pushed(IterationFrame { collection: collection.clone(), position: position.clone() })
            }
            _ => path,
        })
    }

    /// The included frames, outermost first, as legacy inclusion steps.
    pub fn included_steps(&self) -> Vec<ReferenceTrailStep> {
        self.0
            .iter()
            .filter_map(|frame| match frame {
                TrailFrame::Included { file, line } => Some(ReferenceTrailStep { file: file.clone(), line: *line }),
                _ => None,
            })
            .collect()
    }

    /// This trail with `outer`'s frames placed before its own.
    #[must_use]
    pub(crate) fn within(self, outer: Trail) -> Self {
        let mut frames = outer.0;
        frames.extend(self.0);
        Self(frames)
    }

    /// This trail with its iteration frames replaced by `path`'s, appended
    /// innermost.
    #[must_use]
    pub(crate) fn with_iteration(mut self, path: &IterationPath) -> Self {
        self.0.retain(|frame| !matches!(frame, TrailFrame::Iteration { .. }));
        self.0.extend(path.frames().iter().map(TrailFrame::from));
        self
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
