use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use biscuit_terminal::prelude::{
    OrderedList, Prose, TerminalRenderable, RenderableTerminalContent, Table, TableColumn, Terminal, UnorderedList,
};
use cargo_metadata::{Metadata, MetadataCommand, PackageId};
use serde_json::Value;
use sniff::filesystem::{detect_repo, RepoInfo};

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
const INTERRUPTED_MESSAGE: &str = "interrupted by user (Ctrl+C)";
const DRIFT_PREFIX_MARKUP: &str = "<rgb 255,165,0>▌</rgb>";
const AGENT_PREFIX_MARKUP: &str = "<blue>▌</blue>";

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
    agent_override: Option<Agent>,
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

#[derive(Debug)]
struct Ui {
    total_steps: usize,
    term: Terminal,
    drift_prefix: String,
}

#[derive(Clone, Debug)]
struct Cancellation {
    sigint_count: Arc<AtomicUsize>,
}

impl Cancellation {
    fn install() -> Result<Self> {
        let sigint_count = Arc::new(AtomicUsize::new(0));
        let captured = Arc::clone(&sigint_count);
        ctrlc::set_handler(move || {
            captured.fetch_add(1, Ordering::SeqCst);
        })
        .context("failed to install Ctrl+C handler")?;

        Ok(Self { sigint_count })
    }

    fn hits(&self) -> usize {
        self.sigint_count.load(Ordering::SeqCst)
    }

    fn is_cancelled(&self) -> bool {
        self.hits() > 0
    }
}

impl Ui {
    fn new(total_steps: usize) -> Self {
        let term = Terminal::new();
        let drift_prefix = render_prefix(&term, DRIFT_PREFIX_MARKUP);
        Self {
            total_steps,
            term,
            drift_prefix,
        }
    }

    fn banner(&self, package_area: &str, agent: &str) {
        self.print_line("");
        self.print_line(&self.prose(&format!(
            "<b>drift :: {package_area} (docs sync + skill refresh + CLAUDE.md review)</b>"
        )));
        self.print_line(&self.prose(&format!("<dim>agent={agent}  mode=non-interactive</dim>")));
        self.print_line("");
    }

    fn stage(&self, index: usize, label: &str) {
        self.print_line(&format!(
            "{} {}",
            self.prose(&format!("<cyan>[{index}/{}]</cyan>", self.total_steps)),
            label
        ));
    }

    fn item(&self, label: &str, value: &str) {
        self.print_line(&format!(
            "  {} {}",
            self.prose(&format!("<dim>{label}:</dim>")),
            value
        ));
    }

    fn doc_list(&self, docs: &[(String, String)]) {
        if docs.is_empty() {
            return;
        }

        let items = docs
            .iter()
            .map(|(display, uri)| {
                RenderableTerminalContent::from(Prose::new(format!("<a href=\"{uri}\">{display}</a>")))
            })
            .collect::<Vec<_>>();
        let list = UnorderedList::from(items).with_bullet("- ");
        let rendered = list.fallback_render(&self.term);
        print_prefixed_stdout(&self.drift_prefix, &rendered);
    }

    fn warn(&self, message: &str) {
        self.print_line(&format!(
            "  {} {}",
            self.prose("<yellow>warning:</yellow>"),
            message
        ));
    }

    fn ok(&self, message: &str) {
        self.print_line(&format!(
            "  {} {}",
            self.prose("<green>ok:</green>"),
            message
        ));
    }

    fn phase_start(&self, phase: &str, agent: &str) {
        self.print_line(&format!(
            "  {} {}",
            self.prose("<cyan>phase:</cyan>"),
            format!("{phase} (agent={agent})")
        ));
        self.print_line(&format!(
            "  {} waiting for agent output...",
            self.prose("<dim>status:</dim>")
        ));
    }

    fn heartbeat(&self, phase: &str, elapsed: Duration) {
        self.print_line(&format!(
            "  {} {phase} running (elapsed {})",
            self.prose("<dim>status:</dim>"),
            format_duration(elapsed)
        ));
    }

    fn phase_done(&self, phase: &str, elapsed: Duration, status: &str, success: bool) {
        let prefix = if success {
            self.prose("<green>completed:</green>")
        } else {
            self.prose("<yellow>failed:</yellow>")
        };
        self.print_line(&format!(
            "  {prefix} {phase} in {} (status={status})",
            format_duration(elapsed)
        ));
    }

    fn prose(&self, text: &str) -> String {
        Prose::new(text).fallback_render(&self.term)
    }

    fn print_line(&self, text: &str) {
        print_prefixed_stdout(&self.drift_prefix, text);
    }
}

fn main() {
    if let Err(err) = run() {
        let term = Terminal::new();
        let prefix = render_prefix(&term, DRIFT_PREFIX_MARKUP);
        let rendered = format!("{err:#}");
        if rendered.contains(INTERRUPTED_MESSAGE) {
            print_prefixed_stderr(&prefix, "drift cancelled by user");
            std::process::exit(130);
        }
        print_prefixed_stderr(&prefix, &format!("drift failed: {rendered}"));
        std::process::exit(1);
    }
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let minutes = total / 60;
    let seconds = total % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn render_prefix(term: &Terminal, markup: &str) -> String {
    Prose::new(markup).fallback_render(term)
}

fn ensure_not_cancelled(cancellation: &Cancellation) -> Result<()> {
    if cancellation.is_cancelled() {
        bail!(INTERRUPTED_MESSAGE);
    }
    Ok(())
}

fn print_prefixed_stdout(prefix: &str, text: &str) {
    if text.is_empty() {
        println!("{prefix}");
        io::stdout().flush().ok();
        return;
    }

    for line in text.lines() {
        if line.is_empty() {
            println!("{prefix}");
        } else {
            println!("{prefix} {line}");
        }
    }
    io::stdout().flush().ok();
}

fn print_prefixed_stderr(prefix: &str, text: &str) {
    if text.is_empty() {
        eprintln!("{prefix}");
        io::stderr().flush().ok();
        return;
    }

    for line in text.lines() {
        if line.is_empty() {
            eprintln!("{prefix}");
        } else {
            eprintln!("{prefix} {line}");
        }
    }
    io::stderr().flush().ok();
}

fn run() -> Result<()> {
    let workflow_start = Instant::now();
    let cancellation = Cancellation::install()?;
    let cli_args = parse_cli_args(env::args())?;
    let agent = cli_args
        .agent_override
        .unwrap_or_else(|| resolve_agent_preference(env::var("PREFER_AGENT").ok().as_deref()));
    let ui = Ui::new(8);

    ensure_not_cancelled(&cancellation)?;
    ui.banner(&cli_args.package_area, agent.command_name());
    ui.stage(1, "Loading workspace metadata");
    let metadata = MetadataCommand::new()
        .exec()
        .context("failed to load Cargo workspace metadata")?;
    let metadata_workspace_root = metadata.workspace_root.clone().into_std_path_buf();
    let workspace_root =
        resolve_workspace_root_for_package_area(&metadata_workspace_root, &cli_args.package_area);

    ui.stage(
        2,
        &format!("Resolving package context for `{}`", cli_args.package_area),
    );
    let package_context = build_package_context(&metadata, &cli_args.package_area);
    let date = current_date()?;
    let log_paths = build_log_paths(&workspace_root, &date, &package_context.library);

    ui.stage(3, "Discovering document targets");
    let readme_targets = collect_readmes(&workspace_root, &cli_args.package_area)?;
    let docs_value = readme_targets.join(" ");
    let args_value = cli_args.extra_docs.join(" ");
    let doc_links = collect_doc_links(&workspace_root, &readme_targets, &cli_args.extra_docs);
    ui.item("Analyzed docs", &format!("{} files", doc_links.len()));
    ui.doc_list(&doc_links);

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
    let docs_summary = run_agent(
        agent,
        &workspace_root,
        &docs_prompt,
        "docs refresh",
        &ui,
        &cancellation,
    )?;
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
    let _ = run_agent(
        agent,
        &workspace_root,
        &skill_prompt,
        "skill refresh",
        &ui,
        &cancellation,
    )?;

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
        &cancellation,
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

    let raw_args = args.collect::<Vec<_>>();
    let mut extra_docs = Vec::new();
    let mut agent_override = None;
    let mut idx = 0usize;

    while idx < raw_args.len() {
        let current = &raw_args[idx];

        if current == "--agent" {
            let value = raw_args.get(idx + 1).ok_or_else(|| {
                anyhow!("missing value for `--agent`; expected `claude` or `codex`")
            })?;
            agent_override = Some(parse_agent(value)?);
            idx += 2;
            continue;
        }

        if let Some(value) = current.strip_prefix("--agent=") {
            agent_override = Some(parse_agent(value)?);
            idx += 1;
            continue;
        }

        if let Some(value) = current
            .strip_prefix("PREFER_AGENT=")
            .or_else(|| current.strip_prefix("prefer_agent="))
        {
            agent_override = Some(parse_agent(value)?);
            idx += 1;
            continue;
        }

        extra_docs.push(current.clone());
        idx += 1;
    }

    Ok(CliArgs {
        package_area,
        agent_override,
        extra_docs,
    })
}

fn parse_agent(value: &str) -> Result<Agent> {
    Agent::from_preference(value).ok_or_else(|| {
        anyhow!("invalid agent `{value}`; expected one of: claude, claude-code, codex")
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

fn resolve_workspace_root_for_package_area(
    metadata_workspace_root: &Path,
    package_area: &str,
) -> PathBuf {
    for candidate_root in metadata_workspace_root.ancestors() {
        let Ok(Some(repo_info)) = detect_repo(candidate_root) else {
            continue;
        };
        if repo_contains_package_area(&repo_info, package_area) {
            return candidate_root.to_path_buf();
        }
    }
    metadata_workspace_root.to_path_buf()
}

fn repo_contains_package_area(repo_info: &RepoInfo, package_area: &str) -> bool {
    let Some(packages) = repo_info.packages.as_ref() else {
        return false;
    };

    let area_prefix = format!("{package_area}/");
    packages.iter().any(|package| {
        package.package_area == package_area
            || package.relative == package_area
            || package.relative.starts_with(&area_prefix)
    })
}

fn collect_doc_links(
    workspace_root: &Path,
    readme_targets: &[String],
    extra_docs: &[String],
) -> Vec<(String, String)> {
    let mut links = Vec::new();
    let mut seen = BTreeSet::new();

    for raw in readme_targets.iter().chain(extra_docs.iter()) {
        let Some((display, uri)) = doc_target_link(workspace_root, raw) else {
            continue;
        };
        if seen.insert(display.clone()) {
            links.push((display, uri));
        }
    }

    links
}

fn doc_target_link(workspace_root: &Path, raw_target: &str) -> Option<(String, String)> {
    let trimmed = raw_target.trim();
    if trimmed.is_empty() {
        return None;
    }

    let target = trimmed.strip_prefix('@').unwrap_or(trimmed);
    if target.is_empty() {
        return None;
    }

    let path = Path::new(target);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };
    let absolute = fs::canonicalize(&absolute).unwrap_or(absolute);

    let display = absolute
        .strip_prefix(workspace_root)
        .map(path_to_slash_string)
        .unwrap_or_else(|_| target.replace('\\', "/"));
    let uri = file_uri(&absolute);

    Some((display, uri))
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StdoutMode {
    MarkdownText,
    ClaudeStreamJson,
}

fn run_agent(
    agent: Agent,
    workspace_root: &Path,
    prompt: &str,
    phase: &str,
    ui: &Ui,
    cancellation: &Cancellation,
) -> Result<String> {
    ensure_not_cancelled(cancellation)?;
    let mut command = Command::new(agent.command_name());
    command.current_dir(workspace_root);

    match agent {
        Agent::ClaudeCode => {
            command.env_remove("ANTHROPIC_API_KEY");
            command
                .arg("--dangerously-skip-permissions")
                .arg("--verbose")
                .arg("--output-format")
                .arg("stream-json")
                .arg("--include-partial-messages")
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

    let stdout_mode = match agent {
        Agent::ClaudeCode => StdoutMode::ClaudeStreamJson,
        Agent::Codex => StdoutMode::MarkdownText,
    };
    let stdout_thread = spawn_stdout_thread(stdout_handle, stdout_mode);
    let stderr_thread = spawn_stderr_thread(stderr_handle);

    let start = Instant::now();
    let mut last_heartbeat = start;
    let mut cancellation_logged = false;
    let mut termination_requested = false;
    let status = loop {
        let hits = cancellation.hits();
        if hits >= 2 {
            ui.warn("Second interrupt received; exiting immediately.");
            std::process::exit(130);
        }

        if hits >= 1 && !termination_requested {
            if !cancellation_logged {
                ui.warn("Interrupt received; stopping active agent run...");
                cancellation_logged = true;
            }
            match child.kill() {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::InvalidInput => {}
                Err(err) => ui.warn(&format!("Failed to stop agent process cleanly: {err}")),
            }
            termination_requested = true;
        }

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

    if termination_requested {
        bail!(INTERRUPTED_MESSAGE);
    }

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
    agent_prefix: String,
    pending_table_lines: Vec<String>,
    pending_list_lines: Vec<String>,
}

impl MarkdownStreamRenderer {
    fn new() -> Self {
        let term = Terminal::new();
        let agent_prefix = render_prefix(&term, AGENT_PREFIX_MARKUP);
        Self {
            term,
            agent_prefix,
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
            print_prefixed_stdout(&self.agent_prefix, &rendered_table);
        } else {
            for line in &self.pending_table_lines {
                self.print_markdownish_line(line, true);
            }
        }

        self.pending_table_lines.clear();
    }

    fn flush_pending_list(&mut self) {
        if self.pending_list_lines.is_empty() {
            return;
        }

        if let Some(rendered_list) =
            render_markdown_list_block(&self.pending_list_lines, &self.term)
        {
            print_prefixed_stdout(&self.agent_prefix, &rendered_list);
        } else {
            for line in &self.pending_list_lines {
                self.print_markdownish_line(line, true);
            }
        }

        self.pending_list_lines.clear();
    }

    fn print_markdownish_line(&self, line: &str, had_newline: bool) {
        if line.is_empty() {
            if had_newline {
                print_prefixed_stdout(&self.agent_prefix, "");
            }
            return;
        }

        let rendered = render_markdownish_line(line, &self.term);
        if had_newline {
            print_prefixed_stdout(&self.agent_prefix, &rendered);
        } else {
            print_prefixed_stdout(&self.agent_prefix, &rendered);
        }
    }
}

fn spawn_stdout_thread<R>(reader: R, mode: StdoutMode) -> thread::JoinHandle<Result<String>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || -> Result<String> {
        match mode {
            StdoutMode::MarkdownText => capture_markdown_stdout(reader),
            StdoutMode::ClaudeStreamJson => capture_claude_stream_json_stdout(reader),
        }
    })
}

fn capture_markdown_stdout<R>(reader: R) -> Result<String>
where
    R: Read,
{
    let mut renderer = MarkdownStreamRenderer::new();
    let term = Terminal::new();
    let agent_prefix = render_prefix(&term, AGENT_PREFIX_MARKUP);
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
        safe_renderer_process_line(&mut renderer, &line, &agent_prefix);
    }

    safe_renderer_finish(&mut renderer, &agent_prefix);
    Ok(captured)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ClaudeStreamEvent {
    status_lines: Vec<String>,
    text_lines: Vec<String>,
    result_text: Option<String>,
}

fn capture_claude_stream_json_stdout<R>(reader: R) -> Result<String>
where
    R: Read,
{
    let mut renderer = MarkdownStreamRenderer::new();
    let mut buffered = BufReader::new(reader);
    let term = Terminal::new();
    let agent_prefix = render_prefix(&term, AGENT_PREFIX_MARKUP);
    let mut line = String::new();
    let mut raw_captured = String::new();
    let mut summary = String::new();
    let mut last_status = String::new();
    let mut last_text = String::new();

    loop {
        line.clear();
        let read = buffered
            .read_line(&mut line)
            .context("failed reading claude stream-json output")?;
        if read == 0 {
            break;
        }

        raw_captured.push_str(&line);
        let raw = line.trim_end_matches(['\n', '\r']);
        if raw.trim().is_empty() {
            continue;
        }

        if let Some(event) = parse_claude_stream_json_line(raw) {
            for status in event.status_lines {
                let status = status.trim().to_owned();
                if status.is_empty() || status == last_status {
                    continue;
                }
                last_status = status.clone();
                print_prefixed_stdout(&agent_prefix, &format!("status: {status}"));
            }

            for text in event.text_lines {
                let text = text.trim_end().to_owned();
                if text.is_empty() || text == last_text {
                    continue;
                }
                last_text = text.clone();
                safe_renderer_process_line(&mut renderer, &format!("{text}\n"), &agent_prefix);
                summary.push_str(&text);
                summary.push('\n');
            }

            if let Some(result) = event.result_text {
                let result = result.trim().to_owned();
                if !result.is_empty() && result != last_text {
                    last_text = result.clone();
                    safe_renderer_process_line(
                        &mut renderer,
                        &format!("{result}\n"),
                        &agent_prefix,
                    );
                    summary.push_str(&result);
                    summary.push('\n');
                }
            }
            continue;
        }

        safe_renderer_process_line(&mut renderer, &line, &agent_prefix);
        summary.push_str(raw);
        summary.push('\n');
    }

    safe_renderer_finish(&mut renderer, &agent_prefix);

    if summary.trim().is_empty() {
        Ok(raw_captured)
    } else {
        Ok(summary)
    }
}

fn safe_renderer_process_line(
    renderer: &mut MarkdownStreamRenderer,
    line: &str,
    agent_prefix: &str,
) {
    let line_owned = line.to_owned();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        renderer.process_line(&line_owned);
    }));
    if result.is_err() {
        let raw = line_owned.trim_end_matches(['\n', '\r']);
        if raw.is_empty() {
            print_prefixed_stdout(agent_prefix, "");
        } else {
            print_prefixed_stdout(agent_prefix, raw);
        }
        print_prefixed_stdout(
            agent_prefix,
            "warning: markdown rendering failed for one line; continued with raw output",
        );
    }
}

fn safe_renderer_finish(renderer: &mut MarkdownStreamRenderer, agent_prefix: &str) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        renderer.finish();
    }));
    if result.is_err() {
        print_prefixed_stdout(
            agent_prefix,
            "warning: markdown renderer failed during flush; continuing",
        );
    }
}

fn parse_claude_stream_json_line(line: &str) -> Option<ClaudeStreamEvent> {
    let value: Value = serde_json::from_str(line).ok()?;
    Some(parse_claude_stream_json_value(&value))
}

fn parse_claude_stream_json_value(value: &Value) -> ClaudeStreamEvent {
    let mut event = ClaudeStreamEvent::default();
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let subtype = value.get("subtype").and_then(Value::as_str);

    match event_type {
        "system" => {
            if let Some(kind) = subtype {
                event
                    .status_lines
                    .push(format!("claude system event `{kind}`"));
            } else {
                event.status_lines.push("claude system event".to_owned());
            }
        }
        "result" => {
            let mut details = Vec::new();
            if let Some(kind) = subtype {
                details.push(kind.to_owned());
            }
            if let Some(turns) = value.get("num_turns").and_then(Value::as_u64) {
                details.push(format!("turns={turns}"));
            }
            if let Some(duration_ms) = value.get("duration_ms").and_then(Value::as_u64) {
                details.push(format!("duration={duration_ms}ms"));
            }
            if let Some(cost_usd) = json_value_as_f64(
                value
                    .get("total_cost_usd")
                    .or_else(|| value.get("cost_usd"))
                    .unwrap_or(&Value::Null),
            ) {
                details.push(format!("cost=${cost_usd:.4}"));
            }
            if !details.is_empty() {
                event
                    .status_lines
                    .push(format!("claude result ({})", details.join(", ")));
            }
            if value.get("is_error").and_then(Value::as_bool) == Some(true) {
                event
                    .status_lines
                    .push("claude reported an error".to_owned());
            }
            if let Some(result_text) = value.get("result").and_then(Value::as_str) {
                let trimmed = result_text.trim();
                if !trimmed.is_empty() {
                    event.result_text = Some(trimmed.to_owned());
                }
            }
        }
        "assistant" => {
            event.status_lines.push("assistant message".to_owned());
        }
        _ => {}
    }

    if let Some(error) = value.get("error").and_then(Value::as_str) {
        let trimmed = error.trim();
        if !trimmed.is_empty() {
            event.status_lines.push(format!("claude error: {trimmed}"));
        }
    }

    if let Some(message) = value.get("message") {
        extract_message_text_and_status(message, &mut event);
    }
    if let Some(partial_message) = value.get("partial_message") {
        extract_message_text_and_status(partial_message, &mut event);
    }
    if let Some(delta_text) = value
        .get("delta")
        .and_then(|delta| delta.get("text"))
        .and_then(Value::as_str)
    {
        append_non_empty_lines(&mut event.text_lines, delta_text);
    }
    if let Some(top_level_text) = value.get("text").and_then(Value::as_str) {
        append_non_empty_lines(&mut event.text_lines, top_level_text);
    }

    event
}

fn extract_message_text_and_status(message: &Value, event: &mut ClaudeStreamEvent) {
    if let Some(content) = message.get("content").and_then(Value::as_array) {
        for block in content {
            extract_content_block(block, event);
        }
        return;
    }

    if let Some(text) = message.get("text").and_then(Value::as_str) {
        append_non_empty_lines(&mut event.text_lines, text);
    }
}

fn extract_content_block(block: &Value, event: &mut ClaudeStreamEvent) {
    let block_type = block
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();

    match block_type {
        "text" | "thinking" => {
            if let Some(text) = block.get("text").and_then(Value::as_str) {
                append_non_empty_lines(&mut event.text_lines, text);
            }
        }
        "tool_use" => {
            let name = block
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let detail = block
                .get("input")
                .and_then(summarize_tool_input)
                .unwrap_or_default();
            if detail.is_empty() {
                event.status_lines.push(format!("tool call: {name}"));
            } else {
                event
                    .status_lines
                    .push(format!("tool call: {name} ({detail})"));
            }
        }
        "tool_result" => {
            if block.get("is_error").and_then(Value::as_bool) == Some(true) {
                event.status_lines.push("tool result: error".to_owned());
            } else {
                event.status_lines.push("tool result: ok".to_owned());
            }
        }
        _ => {
            if let Some(text) = block.get("text").and_then(Value::as_str) {
                append_non_empty_lines(&mut event.text_lines, text);
            }
        }
    }
}

fn summarize_tool_input(input: &Value) -> Option<String> {
    if let Some(command) = input.get("command").and_then(Value::as_str) {
        return Some(format!("command `{}`", inline_excerpt(command, 80)));
    }
    if let Some(file_path) = input.get("file_path").and_then(Value::as_str) {
        return Some(format!("file `{file_path}`"));
    }
    if let Some(pattern) = input.get("pattern").and_then(Value::as_str) {
        return Some(format!("pattern `{}`", inline_excerpt(pattern, 80)));
    }
    None
}

fn append_non_empty_lines(lines: &mut Vec<String>, text: &str) {
    for raw in text.lines() {
        let trimmed = raw.trim_end();
        if trimmed.trim().is_empty() {
            continue;
        }
        lines.push(trimmed.to_owned());
    }
}

fn inline_excerpt(value: &str, max_chars: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max_chars {
        return collapsed;
    }

    let suffix = "...";
    let budget = max_chars.saturating_sub(suffix.len());
    format!("{}{}", truncate_chars(&collapsed, budget), suffix)
}

fn json_value_as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|value| value as f64))
        .or_else(|| value.as_u64().map(|value| value as f64))
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
}

fn spawn_stderr_thread<R>(reader: R) -> thread::JoinHandle<Result<String>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || -> Result<String> {
        let term = Terminal::new();
        let agent_prefix = render_prefix(&term, AGENT_PREFIX_MARKUP);
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
            let raw = line.trim_end_matches(['\n', '\r']);
            print_prefixed_stderr(&agent_prefix, raw);
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

    let table = Table::new().with_columns(columns).with_data(data);
    Some(table.fallback_render(term))
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
            RenderableTerminalContent::String(text) => output.push_str(&text),
            RenderableTerminalContent::Component(component) => {
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
) -> Option<(RenderableTerminalContent, usize)> {
    let mut items: Vec<RenderableTerminalContent> = Vec::new();

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
        ListKind::Ordered => RenderableTerminalContent::from(OrderedList::from(items)),
        ListKind::Unordered => {
            RenderableTerminalContent::from(UnorderedList::from(items).with_bullet("- "))
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
                agent_override: None,
                extra_docs: vec![
                    "@claudine/docs/extra.md".to_owned(),
                    "@claudine/docs/more.md".to_owned()
                ],
            }
        );
    }

    #[test]
    fn parse_cli_args_extracts_agent_from_assignment() {
        let args = vec![
            "drift".to_owned(),
            "claudine".to_owned(),
            "PREFER_AGENT=codex".to_owned(),
            "@claudine/docs/extra.md".to_owned(),
        ];
        let parsed = parse_cli_args(args.into_iter()).unwrap();
        assert_eq!(
            parsed,
            CliArgs {
                package_area: "claudine".to_owned(),
                agent_override: Some(Agent::Codex),
                extra_docs: vec!["@claudine/docs/extra.md".to_owned()],
            }
        );
    }

    #[test]
    fn parse_cli_args_extracts_agent_from_flag() {
        let args = vec![
            "drift".to_owned(),
            "claudine".to_owned(),
            "--agent".to_owned(),
            "codex".to_owned(),
            "@claudine/docs/extra.md".to_owned(),
        ];
        let parsed = parse_cli_args(args.into_iter()).unwrap();
        assert_eq!(
            parsed,
            CliArgs {
                package_area: "claudine".to_owned(),
                agent_override: Some(Agent::Codex),
                extra_docs: vec!["@claudine/docs/extra.md".to_owned()],
            }
        );
    }

    #[test]
    fn parse_cli_args_rejects_invalid_agent_value() {
        let args = vec![
            "drift".to_owned(),
            "claudine".to_owned(),
            "--agent".to_owned(),
            "bad-agent".to_owned(),
        ];
        let err = parse_cli_args(args.into_iter()).unwrap_err();
        assert!(err.to_string().contains("invalid agent"));
    }

    #[test]
    fn ensure_not_cancelled_allows_normal_flow() {
        let cancellation = Cancellation {
            sigint_count: Arc::new(AtomicUsize::new(0)),
        };
        assert!(ensure_not_cancelled(&cancellation).is_ok());
    }

    #[test]
    fn ensure_not_cancelled_reports_interrupt() {
        let cancellation = Cancellation {
            sigint_count: Arc::new(AtomicUsize::new(1)),
        };
        let err = ensure_not_cancelled(&cancellation).unwrap_err();
        assert!(err.to_string().contains(INTERRUPTED_MESSAGE));
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
    fn resolve_workspace_root_walks_ancestors_via_sniff_repo_detection() {
        let test_root = unique_temp_root();
        let repo_root = test_root.join("workspace");
        let package_area_root = repo_root.join("schematic");
        let package_root = package_area_root.join("define");
        fs::create_dir_all(&package_root).unwrap();

        fs::write(
            repo_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"schematic/define\"]\n",
        )
        .unwrap();
        fs::write(
            package_area_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"define\"]\n",
        )
        .unwrap();
        fs::write(
            package_root.join("Cargo.toml"),
            "[package]\nname = \"schematic-define\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        let resolved = resolve_workspace_root_for_package_area(&package_area_root, "schematic");
        assert_eq!(resolved, repo_root);

        fs::remove_dir_all(test_root).unwrap();
    }

    #[test]
    fn resolve_workspace_root_falls_back_to_metadata_workspace_root() {
        let test_root = unique_temp_root();
        let repo_root = test_root.join("workspace");
        let package_root = repo_root.join("foo");
        fs::create_dir_all(&package_root).unwrap();

        fs::write(
            repo_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"foo\"]\n",
        )
        .unwrap();
        fs::write(
            package_root.join("Cargo.toml"),
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        let resolved = resolve_workspace_root_for_package_area(&repo_root, "schematic");
        assert_eq!(resolved, repo_root);

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
    fn markdown_table_renderer_avoids_cursor_alignment_sequences() {
        let lines = vec![
            "| Col A | Col B |".to_owned(),
            "|-------|-------|".to_owned(),
            "| one   | two   |".to_owned(),
        ];
        let term = Terminal::new();
        let rendered = render_markdown_table_block(&lines, &term).unwrap_or_default();
        assert!(!rendered.contains("\x1b[1G"));
        assert!(!rendered.contains("\x1b[G"));
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

    #[test]
    fn parse_claude_stream_json_extracts_assistant_text_and_tool_call() {
        let raw = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Reading docs now"},{"type":"tool_use","name":"Read","input":{"file_path":"homelab/README.md"}}]}}"#;
        let event = parse_claude_stream_json_line(raw).unwrap();
        assert!(event
            .status_lines
            .iter()
            .any(|line| line == "assistant message"));
        assert!(event
            .status_lines
            .iter()
            .any(|line| line.contains("tool call: Read")));
        assert!(event
            .text_lines
            .iter()
            .any(|line| line == "Reading docs now"));
    }

    #[test]
    fn parse_claude_stream_json_extracts_result_payload() {
        let raw = r#"{"type":"result","subtype":"success","num_turns":4,"duration_ms":1510,"total_cost_usd":0.0125,"result":"Completed with 3 doc updates."}"#;
        let event = parse_claude_stream_json_line(raw).unwrap();
        assert!(event
            .status_lines
            .iter()
            .any(|line| line.starts_with("claude result")));
        assert_eq!(
            event.result_text,
            Some("Completed with 3 doc updates.".to_owned())
        );
    }

    #[test]
    fn parse_claude_stream_json_extracts_delta_text() {
        let raw = r#"{"type":"assistant","delta":{"text":"partial chunk"}}"#;
        let event = parse_claude_stream_json_line(raw).unwrap();
        assert!(event.text_lines.iter().any(|line| line == "partial chunk"));
    }

    #[test]
    fn parse_claude_stream_json_rejects_non_json_lines() {
        assert!(parse_claude_stream_json_line("not json").is_none());
    }

    fn unique_temp_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("drift-tests-{}-{nanos}", std::process::id()))
    }
}
