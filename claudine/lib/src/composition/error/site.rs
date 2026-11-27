//! Where a sequence error was authored.

use std::path::{Path, PathBuf};

/// Where a sequence error was authored: the file, the authored property in
/// it, and the task it belongs to.
///
/// Every `CompositionError::Sequence…` variant without its own `source_path`
/// and `property` carries one. A root-file error (the document's own
/// `sequence` value, its frontmatter) has no task. A layer that raises an
/// error without knowing its file leaves the site unlocated; the first
/// caller that knows it fills it in with
/// [`CompositionError::locate_sequence`](super::CompositionError::locate_sequence),
/// and a site already located keeps the innermost location.
///
/// A site never guesses: an unknown part stays `None` and is not rendered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SequenceSite {
    /// The file that authored the construct.
    pub file: Option<PathBuf>,
    /// The construct's authored property path in [`Self::file`], zero-based
    /// as written (`sequence[2]`, `sequence[0].prompt`, `tasks[1]` in a group
    /// file); `None` for the file as a whole.
    pub property: Option<String>,
    /// The display name of the task the construct belongs to: its authored
    /// `name`, else its generated label.
    pub task: Option<String>,
    /// The one-based line and column in [`Self::file`]'s frame where the
    /// failure was written, when the layer that raised it proved one (a
    /// parser's coordinate); `None` otherwise. Kept only beside the file it
    /// was proven in.
    pub position: Option<(usize, usize)>,
}

impl SequenceSite {
    /// A site whose file the raising layer does not know.
    #[must_use]
    pub fn unlocated() -> Self {
        Self::default()
    }

    /// The file `path` as a whole: a root-file error.
    #[must_use]
    pub fn file(path: &Path) -> Self {
        Self { file: Some(path.to_path_buf()), ..Self::default() }
    }

    /// `property` of `path`. An empty `property` is the file as a whole.
    #[must_use]
    pub fn at(path: &Path, property: &str) -> Self {
        Self {
            file: Some(path.to_path_buf()),
            property: (!property.is_empty()).then(|| property.to_string()),
            task: None,
            position: None,
        }
    }

    /// This site, belonging to the task named `task`.
    #[must_use]
    pub fn in_task(mut self, task: impl Into<String>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// This site, proven written at the one-based `line` and `column` of its
    /// file.
    #[must_use]
    pub fn at_position(mut self, line: usize, column: usize) -> Self {
        self.position = Some((line, column));
        self
    }

    /// Fill what this site does not know: the file and property together,
    /// and the task. A part already known is kept.
    pub fn fill(&mut self, file: &Path, property: &str, task: Option<&str>) {
        if self.file.is_none() {
            self.file = Some(file.to_path_buf());
            if self.property.is_none() && !property.is_empty() {
                self.property = Some(property.to_string());
            }
        }
        if self.task.is_none() {
            self.task = task.map(str::to_string);
        }
    }
}
