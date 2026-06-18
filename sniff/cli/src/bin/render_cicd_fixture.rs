//! Test-only binary that renders a fixture CI/CD run table through the real
//! [`render_cicd`] path.
//!
//! Level 2 tests run this inside a real terminal and capture the pane to verify
//! the status-cell styling survives the terminal's display pipeline:
//! green `✓` for success, red `✗` for failure, dim `⊘` for skipped/cancelled,
//! and dim status text for in-flight runs.

use biscuit_terminal::terminal::Terminal;
use sniff::remote::CiCdInfo;
use sniff_cli::output::render_cicd;

fn run(name: &str, status: &str, conclusion: Option<&str>) -> CiCdInfo {
    CiCdInfo {
        provider: "GitHub Actions".to_string(),
        config_path: None,
        name: name.to_string(),
        status: status.to_string(),
        conclusion: conclusion.map(str::to_string),
        html_url: None,
        // A fixed, clearly-past timestamp marks each entry as run-shaped so
        // `render_cicd` chooses table mode over the presence-only list.
        started_at: Some("2020-01-01T00:00:00Z".to_string()),
        head_branch: Some("main".to_string()),
        event: Some("push".to_string()),
    }
}

fn main() {
    let runs = vec![
        run("Build", "completed", Some("success")),
        run("Lint", "completed", Some("failure")),
        run("Deploy", "completed", Some("skipped")),
        run("Release", "queued", None),
    ];
    let terminal = Terminal::default();
    print!("{}", render_cicd(&runs, &terminal));
}
