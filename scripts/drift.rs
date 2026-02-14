use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::IsTerminal;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use biscuit_terminal::prelude::{
    OrderedList, Prose, Renderable, RenderableContent, Table, TableColumn, Terminal, UnorderedList,
};
use cargo_metadata::{Metadata, MetadataCommand, PackageId};

const AGENT_SKILLS_PROMPT: &str = ".ai/prompts/agent-skills.md";
const REFRESH_DOCUMENTATION_CODEX_PROMPT: &str = ".ai/prompts/refresh_documentation_codex.md";
const REFRESH_DOCUMENTATION_CLAUDE_PROMPT: &str = ".ai/prompts/refresh_documentation_claude.md";
const CLAUDE_MD_UPDATE_PROMPT: &str = ".ai/prompts/refresh_claude_md.md";
const MAX_SKILL_PROMPT_CHARS: usize = 28_000;
const MAX_SKILL_SUMMARY_LINES: usize = 220;
const MAX_SKILL_SUMMARY_CHARS: usize = 16_000;
const MIN_SKILL_SUMMARY_CHARS: usize = 2_000;
const MAX_CLAUDE_MD_SUMMARY_LINES: usize = 140;
const MAX_CLAUDE_MD_SUMMARY_CHARS: usize = 8_000;
const ANSI_RESET: &str = "\x1b[0m";
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_YELLOW: &str = "\x1b[33m";
const ANSI_CYAN: &str = "\x1b[36m";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Agent {
    ClaudeCode,
    Codex,
}

impl Agent {
    fn from_preference(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "claude" | "claude-code" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    fn docs_template_path(self) -> &'static str {
        match self {
            Self::ClaudeCode => REFRESH_DOCUMENTATION_CLAUDE_PROMPT,
            Self::Codex => REFRESH_DOCUMENTATION_CODEX_PROMPT,
        }
    }

    fn command_name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct CliArgs {
    package_area: String,
    extra_docs: Vec<String>,
}

#[derive(Debug)]
struct PackageContext {
    library: String,
    cli: String,
    library_display_name: String,
    dependency_context: String,
}

#[derive(Debug, Eq, PartialEq)]
struct LogPaths {
    docs_prompt: PathBuf,
    skill_prompt: PathBuf,
    claude_md_prompt: PathBuf,
    docs_summary: PathBuf,
}

#[derive(Clone, Copy, Debug)]
struct Ui {
    color: bool,
    total_steps: usize,
}

impl Ui {
    fn new(total_steps: usize) -> Self {
        Self {
            color: supports_color(),
            total_steps,
        }
    }

    fn banner(&self, package_area: &str, agent: &str) {
        println!();
        println!(
            "{}",
            self.paint(
                ANSI_BOLD,
                &format!("drift :: {package_area} (docs sync + skill refresh + CLAUDE.md review)")
            )
        );
        println!(
            "{}",
            self.paint(ANSI_DIM, &format!("agent={agent}  mode=non-interactive"))
        );
        println!();
    }

    fn stage(&self, index: usize, label: &str) {
        println!(
            "{} {}",
            self.paint(ANSI_CYAN, &format!("[{index}/{}]", self.total_steps)),
            label
        );
    }

    fn item(&self, label: &str, value: &str) {
        println!("  {} {}", self.paint(ANSI_DIM, &format!("{label}:")), value);
    }

    fn warn(&self, message: &str) {
        println!("  {} {}", self.paint(ANSI_YELLOW, "warning:"), message);
    }

    fn ok(&self, message: &str) {
        println!("  {} {}", self.paint(ANSI_GREEN, "ok:"), message);
    }

    fn phase_start(&self, phase: &str, agent: &str) {
        println!(
            "  {} {}",
            self.paint(ANSI_CYAN, "phase:"),
            format!("{phase} (agent={agent})")
        );
        println!(
            "  {} waiting for agent output...",
            self.paint(ANSI_DIM, "status:")
        );
    }

    fn heartbeat(&self, phase: &str, elapsed: Duration) {
        println!(
            "  {} {phase} running (elapsed {})",
            self.paint(ANSI_DIM, "status:"),
            format_duration(elapsed)
        );
    }

    fn phase_done(&self, phase: &str, elapsed: Duration, status: &str, success: bool) {
        let prefix = if success {
            self.paint(ANSI_GREEN, "completed:")
        } else {
            self.paint(ANSI_YELLOW, "failed:")
        };
        println!(
            "  {prefix} {phase} in {} (status={status})",
            format_duration(elapsed)
        );
    }

    fn paint(&self, style: &str, text: &str) -> String {
        if self.color {
            format!("{style}{text}{ANSI_RESET}")
        } else {
            text.to_owned()
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("drift failed: {err:#}");
        std::process::exit(1);
    }
}

fn supports_color() -> bool {
    env::var_os("NO_COLOR").is_none() && io::stdout().is_terminal()
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let minutes = total / 60;
    let seconds = total % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn run() -> Result<()> {
    let workflow_start = Instant::now();
    let cli_args = parse_cli_args(env::args())?;
    let agent = resolve_agent_preference(env::var("PREFER_AGENT").ok().as_deref());
    let ui = Ui::new(8);

    ui.banner(&cli_args.package_area, agent.command_name());
    ui.stage(1, "Loading workspace metadata");
    let metadata = MetadataCommand::new()
        .exec()
        .context("failed to load Cargo workspace metadata")?;
    let workspace_root = metadata.workspace_root.clone().into_std_path_buf();

    ui.stage(
        2,
        &format!("Resolving package context for `{}`", cli_args.package_area),
    );
    let package_context = build_package_context(&metadata, &cli_args.package_area);
    let date = current_date()?;
    let log_paths = build_log_paths(&workspace_root, &date, &package_context.library);

    ui.stage(3, "Discovering README targets");
    let readme_targets = collect_readmes(&workspace_root, &cli_args.package_area)?;
    let docs_value = readme_targets.join(" ");
    let args_value = cli_args.extra_docs.join(" ");
    ui.item("README targets", &format!("{} files", readme_targets.len()));
    ui.item("README list", &docs_value);
    if !args_value.is_empty() {
        ui.item("Extra docs", &args_value);
    }

    ui.stage(4, "Rendering docs refresh prompt");
    let docs_template = fs::read_to_string(workspace_root.join(agent.docs_template_path()))
        .with_context(|| {
            format!(
                "failed reading docs template `{}`",
                agent.docs_template_path()
            )
        })?;
    let docs_prompt = render_template(
        &docs_template,
        &[
            ("DOCS", &docs_value),
            ("ARGS", &args_value),
            ("LIBRARY", &package_context.library),
            ("CLI", &package_context.cli),
            ("LIB_NAME", &package_context.library_display_name),
            ("DEPS", &package_context.dependency_context),
        ],
    );
    write_file(&log_paths.docs_prompt, &docs_prompt)?;
    ui.item(
        "Docs prompt",
        &osc8_file_link(&workspace_root, &log_paths.docs_prompt),
    );
    ui.item(
        "Docs prompt size",
        &format!("{} chars", docs_prompt.chars().count()),
    );
    ui.item(
        "Docs prompt payload",
        "path references + instructions (no document bodies injected by drift)",
    );

    ui.stage(5, "Running docs refresh");
    let docs_summary = run_agent(agent, &workspace_root, &docs_prompt, "docs refresh", &ui)?;
    write_file(&log_paths.docs_summary, &docs_summary)?;
    ui.item(
        "Docs summary",
        &log_paths.docs_summary.display().to_string(),
    );

    ui.stage(6, "Rendering skill refresh prompt");
    let mut summary_budget = MAX_SKILL_SUMMARY_CHARS;
    let mut summary_for_skill =
        condensed_summary_for_skill_prompt(&docs_summary, summary_budget, MAX_SKILL_SUMMARY_LINES);

    let skill_template = fs::read_to_string(workspace_root.join(AGENT_SKILLS_PROMPT))
        .with_context(|| format!("failed reading skill template `{AGENT_SKILLS_PROMPT}`"))?;
    let mut skill_prompt = render_template(
        &skill_template,
        &[
            ("LIBRARY", &package_context.library),
            ("LIB_NAME", &package_context.library_display_name),
            ("DOCS", &docs_value),
            ("SUMMARY", &summary_for_skill),
            ("ARGS", &args_value),
        ],
    );

    while skill_prompt.chars().count() > MAX_SKILL_PROMPT_CHARS
        && summary_budget > MIN_SKILL_SUMMARY_CHARS
    {
        summary_budget = summary_budget.saturating_sub(2_000);
        summary_for_skill = condensed_summary_for_skill_prompt(
            &docs_summary,
            summary_budget,
            MAX_SKILL_SUMMARY_LINES,
        );
        skill_prompt = render_template(
            &skill_template,
            &[
                ("LIBRARY", &package_context.library),
                ("LIB_NAME", &package_context.library_display_name),
                ("DOCS", &docs_value),
                ("SUMMARY", &summary_for_skill),
                ("ARGS", &args_value),
            ],
        );
    }

    if skill_prompt.chars().count() > MAX_SKILL_PROMPT_CHARS {
        bail!(
            "skill prompt is still too large after summary compaction ({} chars)",
            skill_prompt.chars().count()
        );
    }

    if summary_for_skill.chars().count() < docs_summary.chars().count() {
        ui.warn(&format!(
            "Skill summary compacted from {} to {} chars to fit prompt budget.",
            docs_summary.chars().count(),
            summary_for_skill.chars().count()
        ));
    }
    ui.item(
        "Skill prompt size",
        &format!("{} chars", skill_prompt.chars().count()),
    );

    write_file(&log_paths.skill_prompt, &skill_prompt)?;
    ui.item(
        "Skill prompt",
        &log_paths.skill_prompt.display().to_string(),
    );

    ui.stage(7, "Running skill refresh");
    let _ = run_agent(agent, &workspace_root, &skill_prompt, "skill refresh", &ui)?;

    ui.stage(8, "Reviewing CLAUDE.md for drift");
    let claude_md_template = fs::read_to_string(workspace_root.join(CLAUDE_MD_UPDATE_PROMPT))
        .with_context(|| {
            format!("failed reading CLAUDE.md template `{CLAUDE_MD_UPDATE_PROMPT}`")
        })?;
    let claude_md_summary = condensed_summary_for_skill_prompt(
        &docs_summary,
        MAX_CLAUDE_MD_SUMMARY_CHARS,
        MAX_CLAUDE_MD_SUMMARY_LINES,
    );
    let claude_md_prompt = render_template(
        &claude_md_template,
        &[
            ("LIBRARY", &package_context.library),
            ("CLI", &package_context.cli),
            ("LIB_NAME", &package_context.library_display_name),
            ("DOCS", &docs_value),
            ("ARGS", &args_value),
            ("SUMMARY", &claude_md_summary),
        ],
    );
    write_file(&log_paths.claude_md_prompt, &claude_md_prompt)?;
    ui.item(
        "CLAUDE.md prompt",
        &osc8_file_link(&workspace_root, &log_paths.claude_md_prompt),
    );
    ui.item(
        "CLAUDE.md prompt size",
        &format!("{} chars", claude_md_prompt.chars().count()),
    );
    let _ = run_agent(
        agent,
        &workspace_root,
        &claude_md_prompt,
        "claude.md review",
        &ui,
    )?;

    ui.ok(&format!(
        "drift workflow complete in {}",
        format_duration(workflow_start.elapsed())
    ));

    Ok(())
}

fn parse_cli_args(mut args: impl Iterator<Item = String>) -> Result<CliArgs> {
    let _bin = args.next();
    let package_area = args
        .next()
        .ok_or_else(|| anyhow!("usage: drift <package-area> [extra docs ...]"))?;

    if package_area.starts_with('-') {
        bail!("invalid package area `{package_area}`");
    }

    let extra_docs = args.collect();

    Ok(CliArgs {
        package_area,
        extra_docs,
    })
}

fn resolve_agent_preference(value: Option<&str>) -> Agent {
    value
        .and_then(Agent::from_preference)
        .unwrap_or(Agent::ClaudeCode)
}

fn build_package_context(metadata: &Metadata, package_area: &str) -> PackageContext {
    let workspace_package_names = workspace_package_names(metadata);
    let (library, cli) = infer_area_packages(package_area, &workspace_package_names);
    let dependency_lines = workspace_dependency_lines(metadata);
    let dependency_context = dependency_lines
        .get(&library)
        .cloned()
        .unwrap_or_else(|| format!("{library}: (none)"));

    PackageContext {
        library,
        cli,
        library_display_name: format!("{} Library", title_case(package_area)),
        dependency_context,
    }
}

fn workspace_package_names(metadata: &Metadata) -> Vec<String> {
    let workspace_ids: BTreeSet<&PackageId> = metadata.workspace_members.iter().collect();

    metadata
        .packages
        .iter()
        .filter(|pkg| workspace_ids.contains(&pkg.id))
        .map(|pkg| pkg.name.clone())
        .collect()
}

fn infer_area_packages(package_area: &str, workspace_package_names: &[String]) -> (String, String) {
    let mut matching = workspace_package_names
        .iter()
        .filter(|name| *name == package_area || name.starts_with(&format!("{package_area}-")))
        .cloned()
        .collect::<Vec<_>>();
    matching.sort();
    matching.dedup();

    let library = if matching.iter().any(|name| name == package_area) {
        package_area.to_owned()
    } else {
        matching
            .first()
            .cloned()
            .unwrap_or_else(|| package_area.to_owned())
    };

    let cli_name = format!("{package_area}-cli");
    let cli = if matching.iter().any(|name| name == &cli_name) {
        cli_name
    } else {
        library.clone()
    };

    (library, cli)
}

fn workspace_dependency_lines(metadata: &Metadata) -> BTreeMap<String, String> {
    let Some(resolve) = metadata.resolve.as_ref() else {
        return BTreeMap::new();
    };

    let workspace_ids: BTreeSet<&PackageId> = metadata.workspace_members.iter().collect();
    let name_map: BTreeMap<&PackageId, &str> = metadata
        .packages
        .iter()
        .map(|pkg| (&pkg.id, pkg.name.as_str()))
        .collect();

    let mut dependency_lines = BTreeMap::new();

    for node in &resolve.nodes {
        if !workspace_ids.contains(&node.id) {
            continue;
        }

        let from = match name_map.get(&node.id) {
            Some(name) => *name,
            None => continue,
        };

        let mut deps = node
            .deps
            .iter()
            .map(|dep| &dep.pkg)
            .filter(|id| workspace_ids.contains(id))
            .filter_map(|id| name_map.get(id).copied())
            .collect::<Vec<_>>();
        deps.sort();
        deps.dedup();

        let line = if deps.is_empty() {
            format!("{from}: (none)")
        } else {
            format!("{from}: {}", deps.join(", "))
        };
        dependency_lines.insert(from.to_owned(), line);
    }

    dependency_lines
}

fn collect_readmes(workspace_root: &Path, package_area: &str) -> Result<Vec<String>> {
    let package_root = workspace_root.join(package_area);
    if !package_root.exists() {
        bail!(
            "package area `{package_area}` not found at {}",
            package_root.display()
        );
    }

    let mut stack = vec![package_root];
    let mut targets = Vec::new();

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)
            .with_context(|| format!("failed to read directory {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if !is_readme(&path) {
                continue;
            }

            let relative = path.strip_prefix(workspace_root).with_context(|| {
                format!(
                    "failed to strip workspace prefix {} from {}",
                    workspace_root.display(),
                    path.display()
                )
            })?;
            targets.push(format!("@{}", path_to_slash_string(relative)));
        }
    }

    targets.sort();
    targets.dedup();
    Ok(targets)
}

fn is_readme(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("README.md"))
        .unwrap_or(false)
}

fn path_to_slash_string(path: &Path) -> String {
    path.iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn current_date() -> Result<String> {
    let output = Command::new("date")
        .arg("+%F")
        .output()
        .context("failed to execute `date +%F`")?;
    if !output.status.success() {
        bail!("`date +%F` failed");
    }

    let date = String::from_utf8(output.stdout).context("date output was not valid UTF-8")?;
    Ok(date.trim().to_owned())
}

fn build_log_paths(workspace_root: &Path, date: &str, library: &str) -> LogPaths {
    let logs_root = workspace_root.join(".ai/logs");

    LogPaths {
        docs_prompt: logs_root.join(format!("{date}. docs_prompt_for_{library}_docs_prompt.md")),
        skill_prompt: logs_root.join(format!(
            "{date}. skill_prompt_for_{library}_skill_prompt.md"
        )),
        claude_md_prompt: logs_root.join(format!(
            "{date}. claude_md_prompt_for_{library}_claude_md_prompt.md"
        )),
        docs_summary: logs_root.join(format!(
            "{date}. doc_update_summary_for_{library}_docs_summary.md"
        )),
    }
}

fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    replacements
        .iter()
        .fold(template.to_owned(), |rendered, (key, value)| {
            rendered.replace(&format!("{{{{{key}}}}}"), value)
        })
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("failed writing {}", path.display()))?;
    Ok(())
}

fn run_agent(
    agent: Agent,
    workspace_root: &Path,
    prompt: &str,
    phase: &str,
    ui: &Ui,
) -> Result<String> {
    let mut command = Command::new(agent.command_name());
    command.current_dir(workspace_root);

    match agent {
        Agent::ClaudeCode => {
            command.env_remove("ANTHROPIC_API_KEY");
            command
                .arg("--dangerously-skip-permissions")
                .arg("--max-turns")
                .arg("30")
                .arg("-p")
                .arg(prompt);
        }
        Agent::Codex => {
            command.arg("exec").arg(prompt);
        }
    }

    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed running `{}`", agent.command_name()))?;

    ui.phase_start(phase, agent.command_name());

    let stdout_handle = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("failed to capture agent stdout"))?;
    let stderr_handle = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("failed to capture agent stderr"))?;

    let stdout_thread = spawn_stdout_thread(stdout_handle);
    let stderr_thread = spawn_stderr_thread(stderr_handle);

    let start = Instant::now();
    let mut last_heartbeat = start;
    let status = loop {
        if let Some(status) = child.try_wait().context("failed while waiting for agent")? {
            break status;
        }

        if last_heartbeat.elapsed() >= Duration::from_secs(20) {
            ui.heartbeat(phase, start.elapsed());
            last_heartbeat = Instant::now();
        }

        thread::sleep(Duration::from_secs(1));
    };

    let stdout = stdout_thread
        .join()
        .map_err(|_| anyhow!("stdout reader thread panicked"))??;
    let stderr = stderr_thread
        .join()
        .map_err(|_| anyhow!("stderr reader thread panicked"))??;

    let status_label = status
        .code()
        .map_or_else(|| "<signal>".to_owned(), |code| code.to_string());
    ui.phase_done(phase, start.elapsed(), &status_label, status.success());

    if !status.success() {
        let combined = format!("{stdout}\n{stderr}");
        let excerpt = tail_excerpt(&combined, 600);
        if combined.to_ascii_lowercase().contains("prompt is too long") {
            bail!(
                "`{}` exited with status {} during `{}`: prompt is too long.\n\
                 The top-level drift prompt contains paths and instructions; this usually means \
                 internal agent-expanded context exceeded limits.\n\
                 Output excerpt:\n{}",
                agent.command_name(),
                status
                    .code()
                    .map_or_else(|| "<signal>".to_owned(), |code| code.to_string()),
                phase,
                excerpt
            );
        }
        bail!(
            "`{}` exited with status {} during `{}`.\nOutput excerpt:\n{}",
            agent.command_name(),
            status
                .code()
                .map_or_else(|| "<signal>".to_owned(), |code| code.to_string()),
            phase,
            excerpt
        );
    }

    Ok(stdout)
}

#[derive(Debug)]
struct MarkdownStreamRenderer {
    term: Terminal,
    pending_table_lines: Vec<String>,
    pending_list_lines: Vec<String>,
}

impl MarkdownStreamRenderer {
    fn new() -> Self {
        Self {
            term: Terminal::new(),
            pending_table_lines: Vec::new(),
            pending_list_lines: Vec::new(),
        }
    }

    fn process_line(&mut self, line: &str) {
        let raw = line.trim_end_matches(['\n', '\r']);
        let had_newline = line.ends_with('\n');

        if is_markdown_table_line(raw) {
            self.flush_pending_list();
            self.pending_table_lines.push(raw.to_owned());
            return;
        }

        if is_markdown_list_line(raw) {
            self.flush_pending_table();
            self.pending_list_lines.push(raw.to_owned());
            return;
        }

        self.flush_pending_table();
        self.flush_pending_list();
        self.print_markdownish_line(raw, had_newline);
    }

    fn finish(&mut self) {
        self.flush_pending_table();
        self.flush_pending_list();
    }

    fn flush_pending_table(&mut self) {
        if self.pending_table_lines.is_empty() {
            return;
        }

        if let Some(rendered_table) =
            render_markdown_table_block(&self.pending_table_lines, &self.term)
        {
            print!("{rendered_table}");
            if !rendered_table.ends_with('\n') {
                println!();
            }
        } else {
            for line in &self.pending_table_lines {
                self.print_markdownish_line(line, true);
            }
        }

        self.pending_table_lines.clear();
        io::stdout().flush().ok();
    }

    fn flush_pending_list(&mut self) {
        if self.pending_list_lines.is_empty() {
            return;
        }

        if let Some(rendered_list) =
            render_markdown_list_block(&self.pending_list_lines, &self.term)
        {
            print!("{rendered_list}");
            if !rendered_list.ends_with('\n') {
                println!();
            }
        } else {
            for line in &self.pending_list_lines {
                self.print_markdownish_line(line, true);
            }
        }

        self.pending_list_lines.clear();
        io::stdout().flush().ok();
    }

    fn print_markdownish_line(&self, line: &str, had_newline: bool) {
        if line.is_empty() {
            if had_newline {
                println!();
            }
            return;
        }

        let rendered = render_markdownish_line(line, &self.term);
        if had_newline {
            println!("{rendered}");
        } else {
            print!("{rendered}");
        }
        io::stdout().flush().ok();
    }
}

fn spawn_stdout_thread<R>(reader: R) -> thread::JoinHandle<Result<String>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || -> Result<String> {
        let mut renderer = MarkdownStreamRenderer::new();
        let mut buffered = BufReader::new(reader);
        let mut line = String::new();
        let mut captured = String::new();

        loop {
            line.clear();
            let read = buffered
                .read_line(&mut line)
                .context("failed reading agent output stream")?;
            if read == 0 {
                break;
            }

            captured.push_str(&line);
            renderer.process_line(&line);
        }

        renderer.finish();
        Ok(captured)
    })
}

fn spawn_stderr_thread<R>(reader: R) -> thread::JoinHandle<Result<String>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || -> Result<String> {
        let mut buffered = BufReader::new(reader);
        let mut line = String::new();
        let mut captured = String::new();

        loop {
            line.clear();
            let read = buffered
                .read_line(&mut line)
                .context("failed reading agent stderr stream")?;
            if read == 0 {
                break;
            }

            captured.push_str(&line);
            eprint!("{line}");
            io::stderr().flush().ok();
        }

        Ok(captured)
    })
}

fn is_markdown_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() >= 3
}

fn render_markdown_table_block(lines: &[String], term: &Terminal) -> Option<String> {
    if lines.len() < 2 {
        return None;
    }

    let header = parse_markdown_row(&lines[0])?;
    let separator = parse_markdown_row(&lines[1])?;
    if header.is_empty() || header.len() != separator.len() {
        return None;
    }
    if !separator
        .iter()
        .all(|cell| is_markdown_separator_cell(cell))
    {
        return None;
    }

    let mut data = Vec::new();
    for row in &lines[2..] {
        let parsed = parse_markdown_row(row)?;
        if parsed.len() != header.len() {
            return None;
        }
        data.push(
            parsed
                .iter()
                .map(|cell| cleanup_markdown_inline(cell).into())
                .collect::<Vec<_>>(),
        );
    }

    let columns = header
        .iter()
        .map(|cell| TableColumn::new(cleanup_markdown_inline(cell)))
        .collect::<Vec<_>>();

    let table = Table::new()
        .with_columns(columns)
        .with_data(data)
        .prefer_cursor_alignment();
    Some(table.render(Some(term.width())))
}

fn parse_markdown_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return None;
    }

    let content = trimmed.trim_start_matches('|').trim_end_matches('|');
    Some(
        content
            .split('|')
            .map(|cell| cell.trim().to_owned())
            .collect(),
    )
}

fn is_markdown_separator_cell(cell: &str) -> bool {
    let trimmed = cell.trim();
    !trimmed.is_empty() && trimmed.contains('-') && trimmed.chars().all(|ch| ch == '-' || ch == ':')
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ListKind {
    Ordered,
    Unordered,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedListLine {
    indent: usize,
    kind: ListKind,
    text: String,
}

fn is_markdown_list_line(line: &str) -> bool {
    parse_markdown_list_line(line).is_some()
}

fn parse_markdown_list_line(line: &str) -> Option<ParsedListLine> {
    let byte_indent = line
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(line.len());
    let indent = line[..byte_indent].chars().count();
    let trimmed = line[byte_indent..].trim_end();
    if trimmed.is_empty() {
        return None;
    }

    let bullet_prefix = ['-', '*', '+']
        .iter()
        .find_map(|bullet| trimmed.strip_prefix(*bullet))
        .and_then(|rest| rest.strip_prefix(' '));
    if let Some(text) = bullet_prefix {
        return Some(ParsedListLine {
            indent,
            kind: ListKind::Unordered,
            text: text.to_owned(),
        });
    }

    let digit_count = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count > 0 {
        let rest = &trimmed[digit_count..];
        if let Some(text) = rest.strip_prefix(". ") {
            return Some(ParsedListLine {
                indent,
                kind: ListKind::Ordered,
                text: text.to_owned(),
            });
        }
    }

    None
}

fn render_markdown_list_block(lines: &[String], term: &Terminal) -> Option<String> {
    let parsed = lines
        .iter()
        .map(|line| parse_markdown_list_line(line))
        .collect::<Option<Vec<_>>>()?;
    if parsed.is_empty() {
        return None;
    }

    let mut output = String::new();
    let mut idx = 0;
    while idx < parsed.len() {
        let line = &parsed[idx];
        let (component, next_idx) = build_list_component(&parsed, idx, line.indent, line.kind)?;
        match component {
            RenderableContent::String(text) => output.push_str(&text),
            RenderableContent::Component(component) => {
                output.push_str(&component.fallback_render(term))
            }
        }
        if !output.ends_with('\n') {
            output.push('\n');
        }
        idx = next_idx;
    }

    Some(output)
}

fn build_list_component(
    lines: &[ParsedListLine],
    mut idx: usize,
    indent: usize,
    kind: ListKind,
) -> Option<(RenderableContent, usize)> {
    let mut items: Vec<RenderableContent> = Vec::new();

    while idx < lines.len() {
        let line = &lines[idx];
        if line.indent < indent {
            break;
        }

        if line.indent > indent {
            let (nested, next_idx) = build_list_component(lines, idx, line.indent, line.kind)?;
            items.push(nested);
            idx = next_idx;
            continue;
        }

        if line.kind != kind {
            break;
        }

        items.push(cleanup_markdown_inline(&line.text).into());
        idx += 1;
    }

    let component = match kind {
        ListKind::Ordered => RenderableContent::from(OrderedList::from(items)),
        ListKind::Unordered => {
            RenderableContent::from(UnorderedList::from(items).with_bullet("- "))
        }
    };

    Some((component, idx))
}

fn render_markdownish_line(line: &str, term: &Terminal) -> String {
    if line.contains('\u{1b}') {
        return line.to_owned();
    }

    let mut normalized = line.to_owned();
    if let Some(heading) = markdown_heading_text(&normalized) {
        normalized = format!("<b>{heading}</b>");
    } else {
        normalized = markdown_bold_to_prose(&normalized);
        normalized = markdown_inline_code_to_dim(&normalized);
    }

    Prose::new(normalized).fallback_render(term)
}

fn markdown_heading_text(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }

    let title = trimmed[hashes..].trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_owned())
    }
}

fn markdown_bold_to_prose(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut bold_open = false;

    while let Some(ch) = chars.next() {
        if ch == '*' && chars.peek() == Some(&'*') {
            chars.next();
            if bold_open {
                output.push_str("</b>");
            } else {
                output.push_str("<b>");
            }
            bold_open = !bold_open;
            continue;
        }
        output.push(ch);
    }

    if bold_open {
        output.push_str("</b>");
    }

    output
}

fn markdown_inline_code_to_dim(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut code_open = false;

    while let Some(ch) = chars.next() {
        if ch == '`' {
            if code_open {
                output.push_str("</dim>");
            } else {
                output.push_str("<dim>");
            }
            code_open = !code_open;
            continue;
        }
        output.push(ch);
    }

    if code_open {
        output.push_str("</dim>");
    }

    output
}

fn cleanup_markdown_inline(input: &str) -> String {
    let without_ticks = input.replace('`', "");
    let without_double_stars = without_ticks.replace("**", "");
    without_double_stars.trim().to_owned()
}

fn osc8_file_link(workspace_root: &Path, path: &Path) -> String {
    let absolute = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let absolute_uri = file_uri(&absolute);
    let display = path
        .strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned();

    format!("\x1b]8;;{absolute_uri}\x1b\\{display}\x1b]8;;\x1b\\")
}

fn file_uri(path: &Path) -> String {
    let text = path.to_string_lossy();
    format!("file://{}", percent_encode_path_for_uri(&text))
}

fn percent_encode_path_for_uri(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for &byte in path.as_bytes() {
        let is_unreserved =
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/' | b':');
        if is_unreserved {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_upper(byte >> 4));
            encoded.push(hex_upper(byte & 0x0F));
        }
    }
    encoded
}

fn condensed_summary_for_skill_prompt(summary: &str, max_chars: usize, max_lines: usize) -> String {
    let non_empty_lines = summary
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();

    let tail_start = non_empty_lines.len().saturating_sub(max_lines);
    let mut compact = non_empty_lines[tail_start..].join("\n");

    if compact.chars().count() > max_chars {
        compact = truncate_chars(&compact, max_chars.saturating_sub(64));
        compact.push_str("\n\n[drift] Summary truncated to fit skill prompt budget.");
    }

    if compact.is_empty() {
        "[drift] No summary content captured from docs refresh.".to_owned()
    } else {
        compact
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn tail_excerpt(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(max_chars);
    chars[start..].iter().collect::<String>().trim().to_owned()
}

fn hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'A' + (nibble - 10)) as char,
        _ => '0',
    }
}

fn title_case(value: &str) -> String {
    value
        .split(['-', '_'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    format!(
                        "{}{}",
                        first.to_ascii_uppercase(),
                        chars.as_str().to_ascii_lowercase()
                    )
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn parse_cli_args_requires_package_area() {
        let args = vec!["drift".to_owned()];
        let err = parse_cli_args(args.into_iter()).unwrap_err();
        assert!(err.to_string().contains("usage: drift"));
    }

    #[test]
    fn parse_cli_args_extracts_area_and_docs() {
        let args = vec![
            "drift".to_owned(),
            "claudine".to_owned(),
            "@claudine/docs/extra.md".to_owned(),
            "@claudine/docs/more.md".to_owned(),
        ];
        let parsed = parse_cli_args(args.into_iter()).unwrap();
        assert_eq!(
            parsed,
            CliArgs {
                package_area: "claudine".to_owned(),
                extra_docs: vec![
                    "@claudine/docs/extra.md".to_owned(),
                    "@claudine/docs/more.md".to_owned()
                ],
            }
        );
    }

    #[test]
    fn resolve_agent_preference_defaults_to_claude() {
        assert_eq!(resolve_agent_preference(None), Agent::ClaudeCode);
        assert_eq!(
            resolve_agent_preference(Some("unsupported-agent")),
            Agent::ClaudeCode
        );
    }

    #[test]
    fn resolve_agent_preference_supports_codex_and_claude() {
        assert_eq!(resolve_agent_preference(Some("codex")), Agent::Codex);
        assert_eq!(resolve_agent_preference(Some("claude")), Agent::ClaudeCode);
        assert_eq!(
            resolve_agent_preference(Some("CLAUDE-CODE")),
            Agent::ClaudeCode
        );
    }

    #[test]
    fn render_template_replaces_all_known_placeholders() {
        let rendered = render_template(
            "A {{ONE}} B {{TWO}} C {{ONE}}",
            &[("ONE", "x"), ("TWO", "y")],
        );
        assert_eq!(rendered, "A x B y C x");
    }

    #[test]
    fn infer_area_packages_prefers_exact_and_cli_variants() {
        let packages = vec![
            "claudine".to_owned(),
            "claudine-cli".to_owned(),
            "darkmatter".to_owned(),
        ];
        let inferred = infer_area_packages("claudine", &packages);
        assert_eq!(inferred, ("claudine".to_owned(), "claudine-cli".to_owned()));
    }

    #[test]
    fn infer_area_packages_falls_back_when_matches_missing() {
        let packages = vec!["other".to_owned()];
        let inferred = infer_area_packages("unknown", &packages);
        assert_eq!(inferred, ("unknown".to_owned(), "unknown".to_owned()));
    }

    #[test]
    fn title_case_handles_kebab_and_snake() {
        assert_eq!(title_case("claudine"), "Claudine");
        assert_eq!(title_case("biscuit-terminal"), "Biscuit Terminal");
        assert_eq!(title_case("so_you_say"), "So You Say");
    }

    #[test]
    fn build_log_paths_uses_existing_naming_convention() {
        let workspace_root = PathBuf::from("/tmp/workspace");
        let paths = build_log_paths(&workspace_root, "2026-02-14", "claudine");
        assert_eq!(
            paths.docs_prompt,
            PathBuf::from(
                "/tmp/workspace/.ai/logs/2026-02-14. docs_prompt_for_claudine_docs_prompt.md"
            )
        );
        assert_eq!(
            paths.skill_prompt,
            PathBuf::from(
                "/tmp/workspace/.ai/logs/2026-02-14. skill_prompt_for_claudine_skill_prompt.md"
            )
        );
        assert_eq!(
            paths.claude_md_prompt,
            PathBuf::from(
                "/tmp/workspace/.ai/logs/2026-02-14. claude_md_prompt_for_claudine_claude_md_prompt.md"
            )
        );
        assert_eq!(
            paths.docs_summary,
            PathBuf::from(
                "/tmp/workspace/.ai/logs/2026-02-14. doc_update_summary_for_claudine_docs_summary.md"
            )
        );
    }

    #[test]
    fn collect_readmes_finds_nested_readmes() {
        let test_root = unique_temp_root();
        let workspace_root = test_root.join("workspace");
        let package_root = workspace_root.join("claudine");
        let nested = package_root.join("docs/sub");
        fs::create_dir_all(&nested).unwrap();
        fs::write(package_root.join("README.md"), "root").unwrap();
        fs::write(nested.join("README.md"), "nested").unwrap();
        fs::write(package_root.join("not-readme.md"), "skip").unwrap();

        let targets = collect_readmes(&workspace_root, "claudine").unwrap();
        assert_eq!(
            targets,
            vec![
                "@claudine/README.md".to_owned(),
                "@claudine/docs/sub/README.md".to_owned()
            ]
        );

        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn percent_encode_path_for_uri_encodes_spaces_and_symbols() {
        let encoded = percent_encode_path_for_uri("/tmp/file name#[x].md");
        assert_eq!(encoded, "/tmp/file%20name%23%5Bx%5D.md");
    }

    #[test]
    fn osc8_file_link_uses_relative_label_and_file_uri() {
        let test_root = unique_temp_root();
        let workspace_root = test_root.join("workspace");
        let logs_path = workspace_root.join(".ai/logs/2026-02-14. docs prompt.md");
        fs::create_dir_all(logs_path.parent().unwrap()).unwrap();
        fs::write(&logs_path, "content").unwrap();

        let link = osc8_file_link(&workspace_root, &logs_path);
        assert!(link.contains(".ai/logs/2026-02-14. docs prompt.md"));
        assert!(link.contains("file:///"));
        assert!(link.contains("%20"));

        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn condensed_summary_uses_tail_and_char_budget() {
        let mut source = String::new();
        for i in 0..400 {
            source.push_str(&format!("line-{i}\n"));
        }
        let compact = condensed_summary_for_skill_prompt(&source, 5_000, 50);
        assert!(compact.contains("line-399"));
        assert!(!compact.contains("line-1"));

        let truncated = condensed_summary_for_skill_prompt(&source, 180, 50);
        assert!(truncated.contains("[drift] Summary truncated"));
        assert!(truncated.chars().count() <= 180 + 64);
    }

    #[test]
    fn truncate_chars_handles_unicode_safely() {
        let value = "✅abc";
        let truncated = truncate_chars(value, 2);
        assert_eq!(truncated, "✅a");
    }

    #[test]
    fn tail_excerpt_returns_end_of_text() {
        let text = "0123456789";
        assert_eq!(tail_excerpt(text, 4), "6789");
        assert_eq!(tail_excerpt(text, 20), "0123456789");
    }

    #[test]
    fn markdown_bold_to_prose_converts_delimiters() {
        assert_eq!(
            markdown_bold_to_prose("a **b** c"),
            "a <b>b</b> c".to_owned()
        );
    }

    #[test]
    fn markdown_heading_text_extracts_title() {
        assert_eq!(
            markdown_heading_text("## Final Report"),
            Some("Final Report".to_owned())
        );
        assert_eq!(markdown_heading_text("no heading"), None);
    }

    #[test]
    fn markdown_table_parser_accepts_basic_table() {
        let lines = vec![
            "| Col A | Col B |".to_owned(),
            "|-------|-------|".to_owned(),
            "| one   | two   |".to_owned(),
        ];
        let term = Terminal::new();
        let rendered = render_markdown_table_block(&lines, &term);
        assert!(rendered.is_some());
    }

    #[test]
    fn parse_markdown_list_line_detects_unordered_and_ordered() {
        let unordered = parse_markdown_list_line("  - item");
        assert_eq!(
            unordered,
            Some(ParsedListLine {
                indent: 2,
                kind: ListKind::Unordered,
                text: "item".to_owned()
            })
        );

        let ordered = parse_markdown_list_line("12. step");
        assert_eq!(
            ordered,
            Some(ParsedListLine {
                indent: 0,
                kind: ListKind::Ordered,
                text: "step".to_owned()
            })
        );
    }

    #[test]
    fn markdown_list_renderer_accepts_nested_lists() {
        let lines = vec![
            "- parent".to_owned(),
            "  - child".to_owned(),
            "- sibling".to_owned(),
        ];
        let term = Terminal::new();
        let rendered = render_markdown_list_block(&lines, &term);
        let output = rendered.unwrap_or_default();
        assert!(output.contains("- parent"));
        assert!(output.contains("- child"));
        assert!(output.contains("- sibling"));
    }

    fn unique_temp_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("drift-tests-{}-{nanos}", std::process::id()))
    }
}
