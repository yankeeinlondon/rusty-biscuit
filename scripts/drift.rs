// This script is responsible for ensuring that:
//
// 1. documentation for a given package is in sync with the actual implementation
// 2. the agent skill for the given package is up to date with the current documentation
//
// During the two steps above the orchestrator will capture a list of "changes made" and using
// that list we'll ask it to evaluate the `CLAUDE.md` file and update it where appropriate

use std::env;

// we need to ensure that all "prompts" live in Markdown documents outside this script
// and we use a form of "templating" based on the "handlebars" templates where variable
// names are surrounded by two curly braces: `{{foobar}}`
const agent_skills_codex: str = "./.ai/prompts/agent-skills.md";
const agent_skills_claude: str = "./.ai/prompts/agent-skills.md";
const refresh_documentation_codex: str = "./.ai/prompts/refresh_documentation_codex.md";
const refresh_documentation_claude: str = "./.ai/prompts/refresh_documentation_claude.md";

pub enum PromptTemplate {
    AgentSkills,
    RefreshDocumentation,
    ClaudeMdUpdate
}


pub enum Agent {
    ClaudeCode,
    Codex
}


/// The various `justfile`'s in each package will call this program with the following
/// CLI signature:
///
/// - `drift <pkg area> ...docs`
///
/// > Note: a "package area" refers to the directories immediately off the root directory
/// > where we tend to "group" up a few packages which are tightly coupled. Most commonly
/// > this is a `lib` and `cli` variant where the package name of the `lib` is the same
/// > as the subdirectory name and the `cli`'s package name tacks on the
fn main() {
    /// The agent preference is determined by the ENV variable `PREFER_AGENT`
    ///
    /// - if set to `codex` then we use the Codex CLI
    /// - if set to `claude` then we use the Claude Code CLI
    /// - if not specified we use the Claude Code CLI
    ///
    /// In all cases, we will call these agents in "non-interactive" mode.
    let agent_preference: Agent = todo!();

    // we're building a poor man's CLI, no need for the overhead of Clap
    let args: Vec<String> = env::args();

    /// The "package area" that we're focusing on
    let pkg_area: String = args.next().unwrap_or_else(|| "<unknown>");

    /// Each "package" area will have a vector of packages which are
    /// the code this package area represents.
    ///
    /// - this information (both packages as well as package areas) is
    /// easily attained by using the sniff library in this monorepo.
    let packages: Vec<String> = todo!();



}
