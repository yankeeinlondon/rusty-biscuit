//! The policy verbs `--allow`, `--deny`, and `--disable`, and the one
//! registry of identifiers that drives their validation, help, and
//! completions on every command that takes them.
//!
//! - `--allow <id|all>` permits a recovery, or explicitly addressed access
//!   (`host=HOST`).
//! - `--deny <id|all>` makes the selected diagnostics blocking.
//! - `--disable <id>` stops a feature from running.
//!
//! Each value is checked as it is parsed ([`PolicyValueParser`]); the
//! selections of one command are then resolved together
//! ([`ResolvedPolicy::resolve`]), which rejects opposite selections before
//! the command reads its input. See `darkmatter/docs/cli/policy.md`.

use std::collections::BTreeSet;
use std::ffi::OsStr;

use clap::builder::TypedValueParser;
use clap_complete::engine::CompletionCandidate;
use darkmatter::markdown::compose::policy::{
    BlockingPolicy, COMPOSE_DIAGNOSTIC_CODES, CTX_OVERRIDE_CODE, INVALID_FRONTMATTER_ASSIGNMENT_CODE,
    MISSING_HYPERLINK_CODE, MISSING_IMAGE_CODE, MISSING_TRANSCLUSION_CODE,
    REASSIGNED_FRONTMATTER_PROPERTY_CODE, SHELL_TIMEOUT_CODE,
};

/// A command that takes policy verbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PolicyCommand {
    Compose,
    /// `md render` and the default `md FILE` rendering.
    Render,
    Clean,
    SchemaValidate,
}

impl PolicyCommand {
    /// The command as a user types it.
    pub fn display(self) -> &'static str {
        match self {
            Self::Compose => "md compose",
            Self::Render => "md render",
            Self::Clean => "md clean",
            Self::SchemaValidate => "md schema validate",
        }
    }

    /// The verbs the command declares.
    pub fn verbs(self) -> &'static [PolicyVerb] {
        match self {
            Self::Compose => &[PolicyVerb::Allow, PolicyVerb::Deny, PolicyVerb::Disable],
            Self::Render => &[PolicyVerb::Deny],
            Self::Clean | Self::SchemaValidate => &[PolicyVerb::Disable],
        }
    }

    /// The diagnostic codes `--deny` accepts beyond the registered
    /// identifiers.
    fn deny_codes(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Compose => COMPOSE_DIAGNOSTIC_CODES,
            Self::Render | Self::Clean | Self::SchemaValidate => &[],
        }
    }
}

/// One of the three policy verbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PolicyVerb {
    Allow,
    Deny,
    Disable,
}

impl PolicyVerb {
    /// The verb's flag.
    pub fn flag(self) -> &'static str {
        match self {
            Self::Allow => "--allow",
            Self::Deny => "--deny",
            Self::Disable => "--disable",
        }
    }
}

/// What a registered identifier selects, which fixes the verbs that take it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PolicyKind {
    /// A recovery: `--allow` permits it, `--deny` makes its diagnostics
    /// blocking (and so does not recover).
    Recovery,
    /// Explicitly addressed access (`host=HOST`): `--allow` only.
    Host,
    /// A feature: `--disable` only.
    Feature,
    /// A family of diagnostics with no recovery: `--deny` only.
    Diagnostics,
}

impl PolicyKind {
    fn accepts(self, verb: PolicyVerb) -> bool {
        matches!(
            (self, verb),
            (Self::Recovery, PolicyVerb::Allow | PolicyVerb::Deny)
                | (Self::Host, PolicyVerb::Allow)
                | (Self::Feature, PolicyVerb::Disable)
                | (Self::Diagnostics, PolicyVerb::Deny)
        )
    }
}

/// One registered identifier.
#[derive(Debug, PartialEq, Eq)]
pub struct PolicyEntry {
    /// The identifier (`host` is written `host=HOST`).
    pub id: &'static str,
    pub kind: PolicyKind,
    /// The commands that accept it.
    pub commands: &'static [PolicyCommand],
    /// The diagnostic codes a recovery governs. Denying one of these codes
    /// selects the recovery, for conflict detection as for blocking.
    pub codes: &'static [&'static str],
    /// What the identifier selects, for help and completions.
    pub summary: &'static str,
}

use PolicyCommand::{Clean, Compose, Render, SchemaValidate};

/// Every registered identifier.
pub const POLICY_REGISTRY: &[PolicyEntry] = &[
    PolicyEntry {
        id: "missing-hyperlinks",
        kind: PolicyKind::Recovery,
        commands: &[Compose],
        codes: &[MISSING_HYPERLINK_CODE],
        summary: "keep the content when a hyperlink target is missing",
    },
    PolicyEntry {
        id: "missing-image-refs",
        kind: PolicyKind::Recovery,
        commands: &[Compose],
        codes: &[MISSING_IMAGE_CODE],
        summary: "keep the content when an image target is missing",
    },
    PolicyEntry {
        id: "missing-transclusions",
        kind: PolicyKind::Recovery,
        commands: &[Compose],
        codes: &[MISSING_TRANSCLUSION_CODE],
        summary: "remove a transclusion directive whose target is missing",
    },
    PolicyEntry {
        id: "ctx-override",
        kind: PolicyKind::Recovery,
        commands: &[Compose],
        codes: &[CTX_OVERRIDE_CODE],
        summary: "replace a non-object `ctx` frontmatter value with the runtime context",
    },
    PolicyEntry {
        id: "invalid-frontmatter-assignment",
        kind: PolicyKind::Recovery,
        commands: &[Compose],
        codes: &[INVALID_FRONTMATTER_ASSIGNMENT_CODE],
        summary: "skip an invalid `set=` assignment on a `::file` directive; valid siblings still apply",
    },
    PolicyEntry {
        id: "reassigned-frontmatter-property",
        kind: PolicyKind::Recovery,
        commands: &[Compose],
        codes: &[REASSIGNED_FRONTMATTER_PROPERTY_CODE],
        summary: "let the rightmost of a `::file` directive's duplicate `set` assignments win",
    },
    PolicyEntry {
        id: "shell-timeout",
        kind: PolicyKind::Recovery,
        commands: &[Compose],
        codes: &[SHELL_TIMEOUT_CODE],
        summary: "replace a timed-out shell command's output with an empty string",
    },
    PolicyEntry {
        id: "host",
        kind: PolicyKind::Host,
        commands: &[Compose],
        codes: &[],
        summary: "allow remote reads from exactly HOST (`host=HOST`)",
    },
    PolicyEntry {
        id: "baseline-schema",
        kind: PolicyKind::Feature,
        commands: &[Compose, Clean, SchemaValidate],
        codes: &[],
        summary: "the default Darkmatter baseline frontmatter schema",
    },
    PolicyEntry {
        id: "schema-triggers",
        kind: PolicyKind::Feature,
        commands: &[Compose, Clean, SchemaValidate],
        codes: &[],
        summary: "schema-trigger discovery and bare-name schema-root lookup",
    },
    PolicyEntry {
        id: "style",
        kind: PolicyKind::Diagnostics,
        commands: &[Compose, Render],
        codes: &[],
        summary: "unknown or deprecated `style:` keys and inline horizontal-rule deprecations (informational notices never block)",
    },
];

/// The registered entry named `id`.
pub fn entry(id: &str) -> Option<&'static PolicyEntry> {
    POLICY_REGISTRY.iter().find(|entry| entry.id == id)
}

/// The entries `verb` accepts on `command`.
pub fn entries(command: PolicyCommand, verb: PolicyVerb) -> impl Iterator<Item = &'static PolicyEntry> {
    POLICY_REGISTRY
        .iter()
        .filter(move |entry| entry.commands.contains(&command) && entry.kind.accepts(verb))
}

/// Whether `verb` takes `all` on `command`: `--disable` never does.
fn accepts_all(command: PolicyCommand, verb: PolicyVerb) -> bool {
    verb != PolicyVerb::Disable && entries(command, verb).next().is_some()
}

/// One parsed `--allow`, `--deny`, or `--disable` value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicySelector {
    All,
    /// A registered identifier, or a diagnostic code its recovery governs.
    Entry {
        entry: &'static PolicyEntry,
        /// The value as written, for messages.
        spelling: String,
    },
    /// `host=HOST`.
    Host(String),
    /// A diagnostic code no recovery governs (`--deny` only).
    Code(&'static str),
}

/// Parses `raw` as a `verb` value on `command`.
///
/// ## Errors
///
/// A message naming why the value is unknown, empty, or unsupported there.
pub fn parse_selector(command: PolicyCommand, verb: PolicyVerb, raw: &str) -> Result<PolicySelector, String> {
    let flag = verb.flag();
    let on = command.display();
    if raw.trim().is_empty() {
        return Err(format!("`{flag}` needs an identifier"));
    }
    if raw == "all" {
        return if verb == PolicyVerb::Disable {
            Err("`--disable` takes no `all`; name each feature to disable".to_string())
        } else if accepts_all(command, verb) {
            Ok(PolicySelector::All)
        } else {
            Err(format!("`{flag} all` does not apply to `{on}`"))
        };
    }
    if let Some(host) = raw.strip_prefix("host=") {
        return match verb {
            PolicyVerb::Allow if entries(command, verb).any(|entry| entry.kind == PolicyKind::Host) => {
                if host.trim().is_empty() {
                    Err("`--allow host=HOST` needs a host".to_string())
                } else {
                    Ok(PolicySelector::Host(host.to_string()))
                }
            }
            PolicyVerb::Allow => Err(format!("`host=HOST` does not apply to `{on}`")),
            PolicyVerb::Deny => Err(
                "`host=HOST` is an allowance only; there is no `--deny host=HOST` (omit the `--allow host=HOST` instead)"
                    .to_string(),
            ),
            PolicyVerb::Disable => Err("`host=HOST` is an allowance, not a feature; use `--allow host=HOST`".to_string()),
        };
    }
    if let Some(entry) = entry(raw) {
        if entry.kind == PolicyKind::Host {
            return Err("write the host to allow as `host=HOST`".to_string());
        }
        if !entry.commands.contains(&command) {
            return Err(format!("`{raw}` does not apply to `{on}`"));
        }
        if !entry.kind.accepts(verb) {
            return Err(match entry.kind {
                PolicyKind::Feature => format!("`{raw}` is a feature; use `--disable {raw}`"),
                PolicyKind::Diagnostics => format!("`{raw}` has no recovery to allow; it can only be denied"),
                PolicyKind::Recovery => format!("`{raw}` is a recovery; use `--allow {raw}` or `--deny {raw}`"),
                PolicyKind::Host => unreachable!("handled above"),
            });
        }
        return Ok(PolicySelector::Entry { entry, spelling: raw.to_string() });
    }
    if let Some(&(code, _)) = command.deny_codes().iter().find(|(code, _)| *code == raw) {
        if verb != PolicyVerb::Deny {
            return Err(format!(
                "`{flag}` accepts registered identifiers only, never a diagnostic code such as `{raw}`"
            ));
        }
        // A code its recovery governs is that recovery, so the opposite
        // spelling of the same selection is caught as a conflict.
        return Ok(match entries(command, verb).find(|entry| entry.codes.contains(&code)) {
            Some(entry) => PolicySelector::Entry { entry, spelling: raw.to_string() },
            None => PolicySelector::Code(code),
        });
    }
    let mut expected: Vec<String> = Vec::new();
    if accepts_all(command, verb) {
        expected.push("all".to_string());
    }
    expected.extend(entries(command, verb).map(display_id));
    let codes = if verb == PolicyVerb::Deny && !command.deny_codes().is_empty() {
        ", or a diagnostic code (see `--help`)"
    } else {
        ""
    };
    Err(format!("unknown identifier `{raw}`; `{flag}` on `{on}` takes {}{codes}", expected.join(", ")))
}

fn display_id(entry: &PolicyEntry) -> String {
    match entry.kind {
        PolicyKind::Host => "host=HOST".to_string(),
        _ => entry.id.to_string(),
    }
}

/// The clap value parser of one verb on one command.
#[derive(Clone, Copy, Debug)]
pub struct PolicyValueParser {
    pub command: PolicyCommand,
    pub verb: PolicyVerb,
}

impl PolicyValueParser {
    pub const fn new(command: PolicyCommand, verb: PolicyVerb) -> Self {
        Self { command, verb }
    }
}

impl TypedValueParser for PolicyValueParser {
    type Value = PolicySelector;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let raw = value.to_str().ok_or_else(|| clap::Error::new(clap::error::ErrorKind::InvalidUtf8).with_cmd(cmd))?;
        parse_selector(self.command, self.verb, raw).map_err(|message| {
            let arg = arg.map_or_else(|| self.verb.flag().to_string(), ToString::to_string);
            cmd.clone().error(clap::error::ErrorKind::InvalidValue, format!("invalid value '{raw}' for '{arg}': {message}"))
        })
    }
}

/// Completion candidates for `verb` on `command`, from the registry.
pub fn complete(command: PolicyCommand, verb: PolicyVerb, current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let mut candidates: Vec<(String, String)> = Vec::new();
    if accepts_all(command, verb) {
        let all = match verb {
            PolicyVerb::Allow => "every recovery (never a host)",
            _ => "every reported warning and error",
        };
        candidates.push(("all".to_string(), all.to_string()));
    }
    for entry in entries(command, verb) {
        let value = match entry.kind {
            PolicyKind::Host => "host=".to_string(),
            _ => entry.id.to_string(),
        };
        candidates.push((value, entry.summary.to_string()));
    }
    if verb == PolicyVerb::Deny {
        candidates.extend(command.deny_codes().iter().map(|(code, help)| (code.to_string(), help.to_string())));
    }
    candidates
        .into_iter()
        .filter(|(value, _)| value.starts_with(current))
        .map(|(value, help)| CompletionCandidate::new(value).help(Some(help.into())))
        .collect()
}

/// The long help of `verb` on `command`, listing what it takes.
pub fn long_help(command: PolicyCommand, verb: PolicyVerb) -> String {
    let lead = match verb {
        PolicyVerb::Allow => {
            "Permit a recovery, or reads from an explicitly named host (repeatable). An allowance controls \
             recovery only: `--deny` can still make the recovered issue's warning blocking. `all` permits \
             every recovery below but never grants a host or authorizes a shell command. Diagnostic codes \
             are not accepted."
        }
        PolicyVerb::Deny => {
            "Make the selected diagnostics blocking (repeatable): the command fails when one is reported, \
             and writes no document. `all` selects every reported warning and error except a recovery \
             named by `--allow`; a specific identifier beats `all` in either verb. Informational notices \
             never block, and denial never re-enables a disabled feature."
        }
        PolicyVerb::Disable => "Stop a feature from running (repeatable). There is no `all`.",
    };
    let mut rows = vec![lead.to_string(), String::new()];
    if accepts_all(command, verb) {
        rows.push(match verb {
            PolicyVerb::Allow => "  all: every recovery listed here".to_string(),
            _ => "  all: every reported warning and error".to_string(),
        });
    }
    for entry in entries(command, verb) {
        rows.push(format!("  {}: {}", display_id(entry), entry.summary));
    }
    if verb == PolicyVerb::Deny && !command.deny_codes().is_empty() {
        rows.push(String::new());
        rows.push("Diagnostic codes:".to_string());
        for (code, help) in command.deny_codes() {
            rows.push(format!("  {code}: {help}"));
        }
    }
    rows.join("\n")
}

/// A usage error of `command`, formatted and exited the way clap reports
/// one (status 2).
pub fn usage_error(command: PolicyCommand, kind: clap::error::ErrorKind, message: impl std::fmt::Display) -> clap::Error {
    use clap::CommandFactory;
    let path: &[&str] = match command {
        PolicyCommand::Compose => &["compose"],
        PolicyCommand::Render => &["render"],
        PolicyCommand::Clean => &["clean"],
        PolicyCommand::SchemaValidate => &["schema", "validate"],
    };
    let mut cmd = crate::args::Cli::command();
    for name in path {
        cmd = cmd.find_subcommand(name).cloned().unwrap_or(cmd);
    }
    cmd.bin_name(command.display()).error(kind, message)
}

/// Resolves `command`'s selections, as a usage error on a conflict.
///
/// `explicit_baseline` names an explicit baseline flag the command was given
/// (`--baseline-schema <PATH>`), which `--disable baseline-schema`
/// contradicts.
///
/// ## Errors
///
/// A [`clap::Error`] of kind `ArgumentConflict`.
pub fn resolve_for(
    command: PolicyCommand,
    allow: &[PolicySelector],
    deny: &[PolicySelector],
    disable: &[PolicySelector],
    explicit_baseline: Option<&str>,
) -> Result<ResolvedPolicy, clap::Error> {
    let conflict = |message: String| usage_error(command, clap::error::ErrorKind::ArgumentConflict, message);
    let policy = ResolvedPolicy::resolve(allow, deny, disable).map_err(conflict)?;
    if let Some(flag) = explicit_baseline
        && policy.disables("baseline-schema")
    {
        return Err(conflict(format!(
            "the argument '--disable baseline-schema' cannot be used with '{flag}'"
        )));
    }
    Ok(policy)
}

/// The resolved selections of one command.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedPolicy {
    allow_all: bool,
    deny_all: bool,
    allowed: BTreeSet<&'static str>,
    denied: BTreeSet<&'static str>,
    denied_codes: BTreeSet<&'static str>,
    disabled: BTreeSet<&'static str>,
    hosts: Vec<String>,
}

impl ResolvedPolicy {
    /// Resolves one command's selections. Repeating a selection is harmless.
    ///
    /// ## Errors
    ///
    /// Opposite selections of equal specificity: `--allow all --deny all`,
    /// or one recovery both allowed and denied under any of its spellings.
    pub fn resolve(
        allow: &[PolicySelector],
        deny: &[PolicySelector],
        disable: &[PolicySelector],
    ) -> Result<Self, String> {
        let mut policy = Self::default();
        let mut spellings: Vec<(PolicyVerb, &'static str, &str)> = Vec::new();
        for (verb, selectors) in [(PolicyVerb::Allow, allow), (PolicyVerb::Deny, deny), (PolicyVerb::Disable, disable)] {
            for selector in selectors {
                match (verb, selector) {
                    (PolicyVerb::Allow, PolicySelector::All) => policy.allow_all = true,
                    (PolicyVerb::Deny, PolicySelector::All) => policy.deny_all = true,
                    (PolicyVerb::Allow, PolicySelector::Host(host)) => {
                        if !policy.hosts.contains(host) {
                            policy.hosts.push(host.clone());
                        }
                    }
                    (PolicyVerb::Deny, PolicySelector::Code(code)) => {
                        policy.denied_codes.insert(code);
                    }
                    (verb, PolicySelector::Entry { entry, spelling }) => {
                        let set = match verb {
                            PolicyVerb::Allow => &mut policy.allowed,
                            PolicyVerb::Deny => &mut policy.denied,
                            PolicyVerb::Disable => &mut policy.disabled,
                        };
                        set.insert(entry.id);
                        spellings.push((verb, entry.id, spelling));
                    }
                    // The value parser accepts no other pairing.
                    (verb, selector) => return Err(format!("`{}` does not take {selector:?}", verb.flag())),
                }
            }
        }
        if policy.allow_all && policy.deny_all {
            return Err("`--allow all` and `--deny all` select opposite policies; keep one".to_string());
        }
        if let Some(id) = policy.allowed.intersection(&policy.denied).next() {
            let spelled = |wanted: PolicyVerb| {
                spellings
                    .iter()
                    .find(|(verb, entry, _)| *verb == wanted && entry == id)
                    .map_or(*id, |(_, _, spelling)| spelling)
            };
            return Err(format!(
                "`--allow {}` and `--deny {}` select the same recovery `{id}`; keep one",
                spelled(PolicyVerb::Allow),
                spelled(PolicyVerb::Deny)
            ));
        }
        Ok(policy)
    }

    /// Whether the recovery `id` applies: allowed by name, or by `all`
    /// unless denied by name.
    pub fn recovers(&self, id: &str) -> bool {
        self.allowed.contains(id) || (self.allow_all && !self.denied.contains(id))
    }

    /// The hosts allowed by `host=HOST`, in first-seen order.
    pub fn hosts(&self) -> &[String] {
        &self.hosts
    }

    /// Whether the feature `id` is disabled.
    pub fn disables(&self, id: &str) -> bool {
        self.disabled.contains(id)
    }

    /// Whether the diagnostics family `id` (such as `style`) blocks.
    pub fn denies(&self, id: &str) -> bool {
        self.deny_all || self.denied.contains(id)
    }

    /// The library policy that decides which reported warnings block.
    pub fn blocking_policy(&self) -> BlockingPolicy {
        let mut policy = BlockingPolicy::default();
        if self.deny_all {
            policy = policy.deny_all();
        }
        for code in self.denied.iter().filter_map(|id| entry(id)).flat_map(|entry| entry.codes) {
            policy = policy.deny_code(*code);
        }
        for code in &self.denied_codes {
            policy = policy.deny_code(*code);
        }
        // A recovery named by `--allow` stays quiet under `--deny all`.
        for code in self.allowed.iter().filter_map(|id| entry(id)).flat_map(|entry| entry.codes) {
            policy = policy.exempt_code(*code);
        }
        policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(command: PolicyCommand, verb: PolicyVerb, raw: &str) -> PolicySelector {
        parse_selector(command, verb, raw).unwrap_or_else(|error| panic!("{raw}: {error}"))
    }

    #[test]
    fn every_entry_is_accepted_by_a_verb_on_each_of_its_commands() {
        for entry in POLICY_REGISTRY {
            for command in entry.commands {
                assert!(
                    command.verbs().iter().any(|verb| entry.kind.accepts(*verb)),
                    "`{}` on {}",
                    entry.id,
                    command.display()
                );
            }
        }
    }

    #[test]
    fn every_recovery_code_is_a_compose_diagnostic_code() {
        for entry in POLICY_REGISTRY {
            for code in entry.codes {
                assert!(COMPOSE_DIAGNOSTIC_CODES.iter().any(|(known, _)| known == code), "{code}");
            }
        }
    }

    #[test]
    fn a_recovery_code_resolves_to_its_recovery() {
        let selector = parse(Compose, PolicyVerb::Deny, MISSING_HYPERLINK_CODE);
        assert!(matches!(selector, PolicySelector::Entry { entry, .. } if entry.id == "missing-hyperlinks"));
        assert_eq!(parse(Compose, PolicyVerb::Deny, "dm.expression.uncertain_call"), PolicySelector::Code("dm.expression.uncertain_call"));
    }

    #[test]
    fn blocking_policy_exempts_only_named_recoveries() {
        let allow = [parse(Compose, PolicyVerb::Allow, "shell-timeout")];
        let deny = [PolicySelector::All];
        let policy = ResolvedPolicy::resolve(&allow, &deny, &[]).unwrap();
        assert!(policy.recovers("shell-timeout"));
        assert!(!policy.recovers("ctx-override"));
        let blocking = policy.blocking_policy();
        use darkmatter::markdown::compose::expression::Severity;
        assert!(!blocking.blocks(Some(SHELL_TIMEOUT_CODE), Severity::Warning));
        assert!(blocking.blocks(Some(CTX_OVERRIDE_CODE), Severity::Warning));
    }
}
