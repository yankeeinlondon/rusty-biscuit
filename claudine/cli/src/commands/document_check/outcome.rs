//! The finished dry run and its two output forms.

use super::*;

/// The finished dry run.
pub(crate) struct Outcome {
    pub(super) file: Option<PathBuf>,
    pub(super) report: FindingReport,
    pub(super) previews: Vec<String>,
    pub(super) tasks: usize,
}

impl Outcome {
    /// The `--json` envelope: `file`, `outcome`, `diagnostics`, `summary`.
    pub(crate) fn envelope(&self) -> Value {
        let counts = self.report.counts();
        serde_json::json!({
            "file": self.file.as_deref().map(to_forward_slash_string),
            "outcome": if self.report.has_problems() { "failure" } else { "success" },
            "diagnostics": self.report.diagnostics(),
            "summary": counts,
        })
    }

    /// The human report, ending in a newline.
    pub(crate) fn human(&self, styled: bool, launch_dir: Option<&Path>) -> String {
        let term = if styled && !crate::log::is_plain() {
            crate::log::terminal()
        } else {
            let mut plain = biscuit_terminal::terminal::Terminal::new_optimistic(80);
            plain.color_depth = biscuit_terminal::discovery::detection::ColorDepth::None;
            plain.is_tty = false;
            plain
        };
        let layout = HumanLayout { width: Some(term.width()), launch_dir: launch_dir.map(Path::to_path_buf) };
        let title = match &self.file {
            Some(file) => format!("Dry run of {} (no commands run)", shown(file, launch_dir)),
            None => "Dry run (no commands run)".to_string(),
        };
        let mut out = format!("{title}\n\n");
        let body = self.report.render_human(&term, &layout);
        if !body.is_empty() {
            out.push_str(&body);
            out.push_str("\n\n");
        }
        out.push_str(&self.summary_line());
        out.push('\n');
        out
    }

    pub(super) fn summary_line(&self) -> String {
        let counts = self.report.counts();
        let not_run =
            self.report.diagnostics().iter().filter(|diagnostic| diagnostic.code == NOT_CHECKED_CODE).count();
        format!(
            "Dry run found {} and {}; {} not run. Previewed {} of {}.",
            plural(counts.errors, "error"),
            plural(counts.warnings, "warning"),
            plural(not_run, "check"),
            self.previews.len(),
            plural(self.tasks, "task"),
        )
    }
}

pub(super) fn plural(count: usize, noun: &str) -> String {
    if count == 1 { format!("1 {noun}") } else { format!("{count} {noun}s") }
}

pub(super) fn shown(path: &Path, launch_dir: Option<&Path>) -> String {
    darkmatter::findings::launch_relative(path, launch_dir)
}
