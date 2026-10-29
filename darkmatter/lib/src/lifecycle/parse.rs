const REMOVED_VALIDATION_KEYS: &[(&str, &str)] = &[
    (
        "pre_checks",
        "use the `initialize` or `start` lifecycle stack instead",
    ),
    (
        "post_checks",
        "use the `success` or `finalize` lifecycle stack instead",
    ),
    (
        "handle",
        "use a lifecycle `shell` action or other lifecycle action instead",
    ),
    (
        "deviate",
        "use a lifecycle `shell` action plus a recovery action (`retry`, `resume`, etc.) instead",
    ),
];

/// Prefix used for subject-specific handler keys that are also removed.
const HANDLE_PREFIX: &str = "handle_";

/// Scan frontmatter top-level keys for removed validation/handler DSL keys.
///
/// Returns the lexicographically first removed key found together with its
/// replacement guidance. This is called from composition preparation before
/// lifecycle event blocks are parsed so the diagnostic names the removed DSL
/// key rather than falling through to generic unknown-field handling.
pub fn scan_removed_validation_keys(
    frontmatter: &serde_json::Value,
) -> Option<(String, &'static str)> {
    let obj = frontmatter.as_object()?;
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort_unstable();
    for key in keys {
        if let Some((_, replacement)) = REMOVED_VALIDATION_KEYS.iter().find(|(k, _)| *k == key) {
            return Some((key.to_string(), *replacement));
        }
        if let Some(suffix) = key.strip_prefix(HANDLE_PREFIX) {
            if !suffix.is_empty() {
                return Some((
                    key.to_string(),
                    "use the `blocked` or `failure` lifecycle recovery actions instead",
                ));
            }
        }
    }
    None
}

/// What the lifecycle parser needs from its caller.
///
/// The parser names no host runtime type, so the catalog it checks literal
/// `effect` names against is supplied here (a host such as Claudine passes
/// its sound-effect catalog). The check, its error
/// ([`LifecycleError::LifecycleUnknownEffect`]), and its timing (parse time,
/// literal names only) are the caller's guarantee; a `{{ … }}` name is checked
/// when the event resolves it.
pub struct LifecycleParseOptions<'a> {
    /// Whether a literal `effect` name is in the effect catalog.
    pub is_known_effect: &'a dyn Fn(&str) -> bool,
    /// The authored frontmatter, whose retained mapping order gives a `set:`
    /// mapping its authored key order. `None` falls back to sorted order.
    pub authored: Option<&'a crate::markdown::Frontmatter>,
    /// What a verb outside the lifecycle verb inventory becomes.
    pub unknown_verbs: UnknownVerbPolicy,
    /// The host's events: which frontmatter keys are lifecycle stacks, and
    /// which Darkmatter spellings the host renamed.
    pub registry: &'a EventRegistry,
}

/// How the parser treats a verb key it does not know.
///
/// Known verbs and the `when`/`then`/`else` structure validate identically
/// under both policies; only an unknown verb's treatment differs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UnknownVerbPolicy {
    /// Reject it with [`LifecycleError::LifecycleUnknownVerb`]: a closed
    /// profile whose every verb is executable.
    #[default]
    Reject,
    /// Keep it as an opaque [`IgnoredAction`] holding only its name and
    /// authored path. Its operand is never parsed, evaluated, or scanned, so
    /// a profile that does not run lifecycle stacks can still read a document
    /// written for one that does.
    Ignore,
    /// [`Self::Ignore`], and the listed verbs of the inventory too: the
    /// profile of a host that runs stacks but provides no service for these
    /// verbs (another host's `say`, `retry`, …). Every other verb parses and
    /// validates as under [`Self::Reject`].
    IgnoreUnprovided(VerbList),
}

impl UnknownVerbPolicy {
    /// Whether `verb` becomes an opaque [`IgnoredAction`] under this policy.
    /// `set` is never ignored.
    pub fn ignores(&self, verb: &str) -> bool {
        if verb == "set" {
            return false;
        }
        match self {
            Self::Reject => false,
            Self::Ignore => !is_known_lifecycle_verb(verb),
            Self::IgnoreUnprovided(unprovided) => !is_known_lifecycle_verb(verb) || unprovided.contains(verb),
        }
    }
}

/// The verbs of [`UnknownVerbPolicy::IgnoreUnprovided`]: a compiled host's
/// static list, or a list read at runtime and owned by every policy that
/// shares it. Equality compares the verbs in order.
#[derive(Debug, Clone)]
pub enum VerbList {
    Static(&'static [&'static str]),
    Shared(std::sync::Arc<[String]>),
}

impl VerbList {
    /// The verbs, in order.
    pub fn iter(&self) -> impl Iterator<Item = &str> + '_ {
        let (statics, owned): (&[&str], &[String]) = match self {
            Self::Static(verbs) => (verbs, &[]),
            Self::Shared(verbs) => (&[], verbs),
        };
        statics.iter().copied().chain(owned.iter().map(String::as_str))
    }

    /// Whether `verb` is listed.
    pub fn contains(&self, verb: &str) -> bool {
        self.iter().any(|listed| listed == verb)
    }
}

impl PartialEq for VerbList {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl Eq for VerbList {}

impl From<&'static [&'static str]> for VerbList {
    fn from(verbs: &'static [&'static str]) -> Self {
        Self::Static(verbs)
    }
}

impl<const N: usize> From<&'static [&'static str; N]> for VerbList {
    fn from(verbs: &'static [&'static str; N]) -> Self {
        Self::Static(verbs)
    }
}

impl From<std::sync::Arc<[String]>> for VerbList {
    fn from(verbs: std::sync::Arc<[String]>) -> Self {
        Self::Shared(verbs)
    }
}

impl From<Vec<String>> for VerbList {
    fn from(verbs: Vec<String>) -> Self {
        Self::Shared(verbs.into())
    }
}

/// Conditionals nest at most this deep. The limit lives only in the parser.
const MAX_CONDITIONAL_DEPTH: usize = 5;

/// The `loop:` keys the looping parser owns. Every other key of a `loop:`
/// block is a lifecycle concern. `fail_fast` stays here although it is
/// removed: the looping parser owns its typed rejection.
pub const LOOP_ITERATION_KEYS: &[&str] = &[
    "while",
    "until",
    "action",
    "actions",
    "max",
    "fail_fast",
    "on_rate_limit",
];

/// Parse every lifecycle stack of `frontmatter`.
///
/// Every event and the `loop:` block's lifecycle concerns parse into an
/// ordered [`LifecycleNode`] stack on [`LifecycleConfig::stacks`]. Literal
/// `effect` names are checked against `options.is_known_effect` here, so an
/// unknown name fails before anything runs.
///
/// ## Errors
///
/// The [`LifecycleError`] authoring family: shape violations
/// ([`LifecycleError::LifecycleStackInvalidShape`]), removed spellings
/// ([`LifecycleError::RemovedGrammar`]), unknown verbs (under
/// [`UnknownVerbPolicy::Reject`]) or parameters, directive placement and
/// order, and [`LifecycleError::LifecycleUnknownEffect`].
pub fn parse_lifecycle_config_with(
    frontmatter: &serde_json::Value,
    source_file: &Path,
    options: &LifecycleParseOptions<'_>,
) -> Result<LifecycleConfig, LifecycleError> {
    crate::note_work!(LifecycleConfigParse);
    let Some(fm_obj) = frontmatter.as_object() else {
        return Ok(LifecycleConfig::default());
    };
    let document_order = options.authored.map_or_else(AuthoredOrder::default, |frontmatter| {
        AuthoredOrder::at_root(std::sync::Arc::new(frontmatter.mapping_orders().clone()))
    });

    refuse_renamed_events(fm_obj, source_file, options.registry)?;
    let mut config = LifecycleConfig::default();
    for signal in [
        LifecycleSignal::Initialize,
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
        LifecycleSignal::Finalize,
        LifecycleSignal::Cancelled,
    ] {
        let Some(event) = options.registry.name_of(signal) else {
            continue;
        };
        let Some(value) = fm_obj.get(event) else {
            continue;
        };
        if signal == LifecycleSignal::Cancelled && value.is_null() {
            return Err(shape_error(source_file, &LifecyclePath::root(event),
                "`cancelled` must be a stack; use `[]` for an empty cancellation opt-in".to_string()));
        }
        let parser = StackParser {
            signal,
            source_file,
            options,
        };
        let nodes = parser.parse_stack(value, &LifecyclePath::root(event), &document_order.child(event))?;
        if signal == LifecycleSignal::Initialize {
            refuse_initialize_shells(&nodes, source_file)?;
        }
        refuse_undeclared_action_types(&nodes, signal, source_file, options.registry)?;
        *config.stack_mut(signal) = nodes;
        if signal == LifecycleSignal::Cancelled {
            config.cancelled_declared = true;
        }
    }

    if let Some(loop_key) = options.registry.name_of(LifecycleSignal::Loop)
        && let Some(loop_value) = fm_obj.get(loop_key)
        && !loop_value.is_null()
    {
        let Some(loop_obj) = loop_value.as_object() else {
            return Err(LifecycleError::LifecycleInvalid {
                property: loop_key.to_string(),
                message: format!("`{loop_key}` must be an object, got {}", json_type_name(loop_value)),
                source_file: source_file.to_path_buf(),
                unknown_field: None,
                expected_fields: Vec::new(),
            });
        };
        let parser = StackParser {
            signal: LifecycleSignal::Loop,
            source_file,
            options,
        };
        config.stacks.loop_gate = parser.parse_loop_concerns(
            loop_obj,
            &LifecyclePath::root(loop_key),
            &document_order.child(loop_key),
        )?;
    }

    Ok(config)
}

/// Parse the lifecycle concerns of a `loop:` block authored at `property`
/// (`loop`, `tasks[2].loop`): the same grammar a document's `loop:` has.
pub fn parse_loop_concerns_with(
    loop_obj: &serde_json::Map<String, serde_json::Value>,
    source_file: &Path,
    property: &str,
    authored: &AuthoredOrder,
    options: &LifecycleParseOptions<'_>,
) -> Result<Vec<LifecycleNode>, LifecycleError> {
    let parser = StackParser {
        signal: LifecycleSignal::Loop,
        source_file,
        options,
    };
    parser.parse_loop_concerns(loop_obj, &LifecyclePath::root(property), authored)
}

/// Under a host that renamed `composed` or `failed`, the Darkmatter spelling
/// is an error naming the host's, checked before any stack parses.
fn refuse_renamed_events(
    frontmatter: &serde_json::Map<String, serde_json::Value>,
    source_file: &Path,
    registry: &EventRegistry,
) -> Result<(), LifecycleError> {
    for event in CoreEvent::ALL {
        let authored = event.darkmatter_name();
        if !frontmatter.contains_key(authored) {
            continue;
        }
        if let KeyResolution::Renamed { authored, replacement } = registry.resolve_key(authored) {
            return Err(LifecycleError::LifecycleRenamedEvent {
                source_path: source_file.to_path_buf(),
                authored,
                replacement,
            });
        }
    }
    Ok(())
}

/// Initialization is shell-free: a `shell` action, or a `set` value that runs
/// a command, is refused in either branch of every conditional, before
/// anything runs.
fn refuse_initialize_shells(nodes: &[LifecycleNode], source_file: &Path) -> Result<(), LifecycleError> {
    let mut offending: Option<&LifecycleAction> = None;
    for_each_action(nodes, &mut |action| {
        if offending.is_none() && runs_a_shell(action) {
            offending = Some(action);
        }
    });
    match offending {
        None => Ok(()),
        Some(action) => Err(LifecycleError::LifecycleActionPlacement {
            source_path: source_file.to_path_buf(),
            property: action.path.to_string(),
            action: "shell".to_string(),
            event: "initialize".to_string(),
        }),
    }
}

/// Every action, in both branches of every conditional, must be of a type
/// the host's registry declares for its event. Ignored actions have no type.
fn refuse_undeclared_action_types(
    nodes: &[LifecycleNode],
    signal: LifecycleSignal,
    source_file: &Path,
    registry: &EventRegistry,
) -> Result<(), LifecycleError> {
    let Some(id) = registry.id_of(signal) else {
        return Ok(());
    };
    let declared = registry.actions(id);
    let mut offending: Option<&LifecycleAction> = None;
    for_each_action(nodes, &mut |action| {
        if offending.is_none() && action_type(action).is_some_and(|kind| !declared.contains(kind)) {
            offending = Some(action);
        }
    });
    match offending {
        None => Ok(()),
        Some(action) => Err(LifecycleError::LifecycleActionPlacement {
            source_path: source_file.to_path_buf(),
            property: action.path.to_string(),
            action: action_verb(action).to_string(),
            event: registry.name(id).to_string(),
        }),
    }
}

/// The [`ActionType`] an action belongs to; an ignored action has none.
pub fn action_type(action: &LifecycleAction) -> Option<ActionType> {
    if runs_a_shell(action) {
        return Some(ActionType::Shell);
    }
    match &action.kind {
        LifecycleActionKind::LifecycleControl(_) => Some(ActionType::Flow),
        LifecycleActionKind::Communication(_) => Some(ActionType::Communicate),
        LifecycleActionKind::Shell(_) => Some(ActionType::Shell),
        LifecycleActionKind::SideEffect(_)
        | LifecycleActionKind::RuntimeSet(_)
        | LifecycleActionKind::ExpressionFunction(_) => Some(ActionType::Expression),
        LifecycleActionKind::Ignored(_) => None,
    }
}

/// The verb an action was authored with.
fn action_verb(action: &LifecycleAction) -> &str {
    match &action.kind {
        LifecycleActionKind::LifecycleControl(control) => control.verb(),
        LifecycleActionKind::Communication(comm) => comm.channel.verb(),
        LifecycleActionKind::Shell(_) => "shell",
        LifecycleActionKind::SideEffect(effect) => &effect.verb,
        LifecycleActionKind::RuntimeSet(_) => "set",
        LifecycleActionKind::ExpressionFunction(func) => &func.function,
        LifecycleActionKind::Ignored(ignored) => &ignored.verb,
    }
}

/// `signal`'s authored name under `registry`, falling back to the engine
/// slot's name for a slot the host does not expose.
pub(crate) fn event_name(registry: &EventRegistry, signal: LifecycleSignal) -> &str {
    registry.name_of(signal).unwrap_or(signal.property_name())
}

/// Parse a task's `setup:`/`teardown:` value into a typed stack rooted at
/// `property`.
pub fn parse_task_action_stack_with(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
    authored: &AuthoredOrder,
    options: &LifecycleParseOptions<'_>,
) -> Result<Vec<LifecycleNode>, LifecycleError> {
    let parser = StackParser {
        signal,
        source_file,
        options,
    };
    // A stack's whole-value `set` `$( … )` is parsed like any other; sequence
    // preflight approves it, and an unapproved one refuses to run at its turn.
    parser.parse_stack(raw, &LifecyclePath::root(property), authored)
}

/// A `side_effect:` task's `set` stores data; it cannot run a command.
///
/// Sequence preflight approves the commands of a task's `setup:`/`teardown:`
/// stacks, not of its `side_effect:` action, so a whole-value `$( … )` there
/// would have no approved bytes. It is refused rather than stored as its own
/// text.
fn refuse_task_set_shell(action: &LifecycleAction, source_file: &Path) -> Result<(), LifecycleError> {
    let LifecycleActionKind::RuntimeSet(set) = &action.kind else {
        return Ok(());
    };
    match set.shell_values().next() {
        None => Ok(()),
        Some((key, _)) => Err(LifecycleError::LifecycleActionInvalidLongForm {
            source_path: source_file.to_path_buf(),
            property: action.path.parent().to_string(),
            action: "set".to_string(),
            message: format!(
                "`{key}` is a whole-value `$( … )`, and a `side_effect:` task's `set` cannot \
                 run a command; use a `shell:` task, or assign it with `set` in the task's \
                 `setup:`/`teardown:` or in the task document's own lifecycle"
            ),
            source: None,
        }),
    }
}

/// Parse a task's `side_effect:` value: one action, written as a one-verb
/// dictionary in short or long form.
///
/// `property` is the source-rooted path of the value itself
/// (`tasks[0].side_effect`); the action is `{property}.{verb}`.
pub fn parse_single_action_with(
    signal: LifecycleSignal,
    raw: &serde_json::Value,
    source_file: &Path,
    property: &str,
    authored_set_order: Option<&[String]>,
    options: &LifecycleParseOptions<'_>,
) -> Result<LifecycleAction, LifecycleError> {
    let path = LifecyclePath::root(property);
    let serde_json::Value::Object(map) = raw else {
        return Err(shape_error(
            source_file,
            &path,
            format!("`{property}` must be a one-verb dictionary, got {}", json_type_name(raw)),
        ));
    };
    let parser = StackParser {
        signal,
        source_file,
        options,
    };
    parser.reject_removed_keys(map, &path)?;
    let verbs = map.keys().filter(|key| key.as_str() != "no_error").count();
    if verbs != 1 {
        return Err(shape_error(
            source_file,
            &path,
            format!("`{property}` holds exactly one action, got {verbs} verbs"),
        ));
    }
    // The caller holds a `set:` mapping's authored key order directly; every
    // other verb parses as a one-verb bundle.
    let action = match map.get("set") {
        Some(_) if map.contains_key("no_error") => {
            return Err(shape_error(
                source_file,
                &path.field("no_error"),
                "`no_error` does not apply to `set`; a `set` failure always stops the stack"
                    .to_string(),
            ));
        }
        Some(value) => parse_runtime_set(value, source_file, &path.field("set"), authored_set_order)?,
        None => parser
            .parse_bundle(map, &path, &AuthoredOrder::default())?
            .pop()
            .expect("one verb parses to one action"),
    };
    refuse_task_set_shell(&action, source_file)?;
    Ok(action)
}

/// Parses one stack's grammar for one event.
struct StackParser<'a> {
    signal: LifecycleSignal,
    source_file: &'a Path,
    options: &'a LifecycleParseOptions<'a>,
}

impl StackParser<'_> {
    /// A stack root: omitted/`null`, a list, or a dictionary of verbs. `[]`
    /// and `{}` are the empty stack.
    fn parse_stack(
        &self,
        value: &serde_json::Value,
        path: &LifecyclePath,
        authored: &AuthoredOrder,
    ) -> Result<Vec<LifecycleNode>, LifecycleError> {
        match value {
            serde_json::Value::Null => Ok(Vec::new()),
            serde_json::Value::Array(items) => self.parse_list(items, path, authored, 0),
            serde_json::Value::Object(map) => self.parse_dictionary(map, path, authored),
            other => Err(shape_error(
                self.source_file,
                path,
                format!(
                    "a lifecycle stack must be a list of actions or a dictionary of verbs, got {}",
                    json_type_name(other)
                ),
            )),
        }
    }

    /// The `loop:` block's lifecycle concerns: loose verb keys (a root
    /// dictionary) or a `gate:` stack, never both, and never `when`.
    ///
    /// Unlike an event, a gate is never `null` or `{}`: an explicit empty gate
    /// is `gate: []`.
    ///
    /// Removed keys are rejected before the iteration controls are filtered
    /// out, so an old `loop.stack` cannot disappear unnoticed.
    fn parse_loop_concerns(
        &self,
        loop_obj: &serde_json::Map<String, serde_json::Value>,
        path: &LifecyclePath,
        authored: &AuthoredOrder,
    ) -> Result<Vec<LifecycleNode>, LifecycleError> {
        // `action`/`actions` at the loop root are iteration controls, so only
        // the other removed keys are checked here, before filtering.
        let without_iteration: serde_json::Map<String, serde_json::Value> = loop_obj
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "action" | "actions"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        self.reject_removed_keys(&without_iteration, path)?;
        if loop_obj.contains_key("when") {
            return Err(shape_error(
                self.source_file,
                &path.field("when"),
                "`when` cannot sit at the `loop:` root; put a guarded item in `gate:`".to_string(),
            ));
        }
        let concerns: serde_json::Map<String, serde_json::Value> = loop_obj
            .iter()
            .filter(|(key, _)| !LOOP_ITERATION_KEYS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if let Some(gate) = concerns.get("gate") {
            if let Some(loose) = concerns.keys().find(|key| key.as_str() != "gate") {
                return Err(shape_error(
                    self.source_file,
                    &path.field(loose.as_str()),
                    format!(
                        "`{path}.{loose}` cannot sit beside `{path}.gate`; a loop spells its lifecycle \
                         concerns as loose verb keys or as a `gate:` list, not both"
                    ),
                ));
            }
            let gate_path = path.field("gate");
            let empty_form = match gate {
                serde_json::Value::Null => Some("`null`"),
                serde_json::Value::Object(map) if map.is_empty() => Some("an empty dictionary"),
                _ => None,
            };
            if let Some(form) = empty_form {
                return Err(shape_error(
                    self.source_file,
                    &gate_path,
                    format!(
                        "`loop.gate` cannot be {form}; write `gate: []` for an explicit empty gate, \
                         or omit `gate`"
                    ),
                ));
            }
            return self.parse_stack(gate, &gate_path, &authored.child("gate"));
        }
        if concerns.is_empty() {
            return Ok(Vec::new());
        }
        self.parse_bundle(&concerns, path, authored)
            .map(|actions| actions.into_iter().map(LifecycleNode::Action).collect())
    }

    /// A list: a stack root, or a `then`/`else` body. `depth` is the number
    /// of conditionals enclosing it.
    ///
    /// Within one list an unconditional directive that ends the event must be
    /// the last item; a guarded item never counts as unconditional, and `prep`
    /// returns, so items may follow it.
    fn parse_list(
        &self,
        items: &[serde_json::Value],
        path: &LifecyclePath,
        authored: &AuthoredOrder,
        depth: usize,
    ) -> Result<Vec<LifecycleNode>, LifecycleError> {
        let mut nodes = Vec::with_capacity(items.len());
        let mut terminal: Option<LifecyclePath> = None;
        for (index, item) in items.iter().enumerate() {
            if let Some(directive) = terminal.take() {
                return Err(LifecycleError::LifecycleActionOrder {
                    source_path: self.source_file.to_path_buf(),
                    property: directive.to_string(),
                });
            }
            for node in self.parse_item(item, &path.index(index), &authored.at(index), depth)? {
                if let LifecycleNode::Action(action) = &node
                    && action.ends_event()
                {
                    terminal = Some(action.path.clone());
                }
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    /// One list item: a bare zero-argument verb (`- stop`), a dictionary
    /// bundle, or a `when` item.
    fn parse_item(
        &self,
        value: &serde_json::Value,
        path: &LifecyclePath,
        authored: &AuthoredOrder,
        depth: usize,
    ) -> Result<Vec<LifecycleNode>, LifecycleError> {
        let map = match value {
            serde_json::Value::String(raw) => {
                return self.parse_bare_verb(raw, path).map(|action| vec![LifecycleNode::Action(action)]);
            }
            serde_json::Value::Object(map) => map,
            other => {
                return Err(shape_error(
                    self.source_file,
                    path,
                    format!(
                        "a stack item must be a verb name or a dictionary, got {}",
                        json_type_name(other)
                    ),
                ));
            }
        };
        self.reject_removed_keys(map, path)?;
        if map.is_empty() {
            return Err(shape_error(self.source_file, path, "a stack item cannot be empty".to_string()));
        }
        let Some(raw_when) = map.get("when") else {
            for key in ["then", "else"] {
                if map.contains_key(key) {
                    return Err(shape_error(
                        self.source_file,
                        &path.field(key),
                        format!("`{key}` requires `when` on the same item"),
                    ));
                }
            }
            return Ok(self
                .parse_bundle(map, path, authored)?
                .into_iter()
                .map(LifecycleNode::Action)
                .collect());
        };

        let level = depth + 1;
        if level > MAX_CONDITIONAL_DEPTH {
            return Err(shape_error(
                self.source_file,
                &path.field("when"),
                format!(
                    "conditionals nest at most {MAX_CONDITIONAL_DEPTH} levels deep; this `when` is \
                     level {level}"
                ),
            ));
        }
        let when = self.parse_when(raw_when, &path.field("when"))?;

        let (then, otherwise) = if let Some(raw_then) = map.get("then") {
            if map.contains_key("no_error") {
                return Err(shape_error(
                    self.source_file,
                    &path.field("no_error"),
                    "`no_error` cannot sit beside `then`/`else`; put it on the actions in the body"
                        .to_string(),
                ));
            }
            if let Some(extra) = map
                .keys()
                .find(|key| !matches!(key.as_str(), "when" | "then" | "else"))
            {
                return Err(shape_error(
                    self.source_file,
                    &path.field(extra.as_str()),
                    format!(
                        "a `when` item with `then` cannot also carry the sibling `{extra}`; move it \
                         into `then`"
                    ),
                ));
            }
            let then = self.parse_body(raw_then, &path.field("then"), &authored.child("then"), level)?;
            let otherwise = match map.get("else") {
                Some(raw_else) => {
                    self.parse_body(raw_else, &path.field("else"), &authored.child("else"), level)?
                }
                None => Vec::new(),
            };
            (then, otherwise)
        } else {
            if map.contains_key("else") {
                return Err(shape_error(
                    self.source_file,
                    &path.field("else"),
                    "`else` requires `then`; write the true branch under `then`".to_string(),
                ));
            }
            let mut bundle = map.clone();
            bundle.remove("when");
            if bundle.keys().all(|key| key == "no_error") {
                return Err(shape_error(
                    self.source_file,
                    path,
                    "a `when` item needs `then` or sibling actions".to_string(),
                ));
            }
            let then = self
                .parse_bundle(&bundle, path, authored)?
                .into_iter()
                .map(LifecycleNode::Action)
                .collect();
            (then, Vec::new())
        };

        Ok(vec![LifecycleNode::Conditional(LifecycleConditional {
            when,
            then,
            otherwise,
            path: path.clone(),
        })])
    }

    /// An explicitly authored `then:`/`else:` body: a non-empty list or a
    /// non-empty dictionary of verbs. `depth` is the enclosing conditional's
    /// level.
    fn parse_body(
        &self,
        value: &serde_json::Value,
        path: &LifecyclePath,
        authored: &AuthoredOrder,
        depth: usize,
    ) -> Result<Vec<LifecycleNode>, LifecycleError> {
        let branch = path.to_string();
        let branch = branch.rsplit('.').next().unwrap_or("then");
        match value {
            serde_json::Value::Array(items) if !items.is_empty() => {
                self.parse_list(items, path, authored, depth)
            }
            serde_json::Value::Object(map) if !map.is_empty() => self.parse_dictionary(map, path, authored),
            serde_json::Value::Null | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                Err(shape_error(
                    self.source_file,
                    path,
                    format!("an explicit `{branch}` must contain at least one action"),
                ))
            }
            other => Err(shape_error(
                self.source_file,
                path,
                format!(
                    "`{branch}` must be a list of actions or a dictionary of verbs, got {}",
                    json_type_name(other)
                ),
            )),
        }
    }

    /// A dictionary of verbs standing for a whole stack or body. It expands
    /// in canonical order and never carries `when`.
    fn parse_dictionary(
        &self,
        map: &serde_json::Map<String, serde_json::Value>,
        path: &LifecyclePath,
        authored: &AuthoredOrder,
    ) -> Result<Vec<LifecycleNode>, LifecycleError> {
        self.reject_removed_keys(map, path)?;
        for key in ["when", "then", "else"] {
            if map.contains_key(key) {
                return Err(shape_error(
                    self.source_file,
                    &path.field(key),
                    format!(
                        "a dictionary of verbs never carries `{key}`; write a guarded list item \
                         (`- when: …`) instead"
                    ),
                ));
            }
        }
        if map.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self
            .parse_bundle(map, path, authored)?
            .into_iter()
            .map(LifecycleNode::Action)
            .collect())
    }

    /// Reject the removed spellings that can appear as keys: `stack`, the
    /// `action:` aggregator and discriminator, and `say_first`.
    fn reject_removed_keys(
        &self,
        map: &serde_json::Map<String, serde_json::Value>,
        path: &LifecyclePath,
    ) -> Result<(), LifecycleError> {
        let removed = if map.contains_key("stack") {
            Some((
                "stack",
                "write the event as an ordered list of actions, e.g. `start: [ { info: \"…\" } ]`"
                    .to_string(),
            ))
        } else if map.contains_key("action") {
            let replacement = match map.get("action") {
                Some(serde_json::Value::String(verb)) if !map.contains_key("when") => format!(
                    "write the verb as the key, e.g. `- {verb}: …`, with named parameters as \
                     `- {verb}: {{ … }}`"
                ),
                _ if map.contains_key("when") => {
                    "put the guarded actions under `then:` (or beside `when` as sibling verbs)"
                        .to_string()
                }
                _ => "list the actions directly, one verb per item, e.g. `- shell: \"…\"`"
                    .to_string(),
            };
            Some(("action", replacement))
        } else if map.contains_key("say_first") {
            Some((
                "say_first",
                "use `say`, and order the list: items run in the order written".to_string(),
            ))
        } else {
            None
        };
        match removed {
            None => Ok(()),
            Some((key, replacement)) => Err(LifecycleError::RemovedGrammar {
                source_path: self.source_file.to_path_buf(),
                path: path.field(key).to_string(),
                removed: key.to_string(),
                replacement,
            }),
        }
    }

    fn parse_when(&self, value: &serde_json::Value, path: &LifecyclePath) -> Result<Expr, LifecycleError> {
        // `when` is always a boolean expression, never routed through the
        // literal-default action-value rule.
        match value {
            serde_json::Value::String(text) => parse_condition(text).map_err(|source| {
                LifecycleError::LifecycleWhenExpressionInvalid {
                    source_path: self.source_file.to_path_buf(),
                    property: path.to_string(),
                    message: format!("`when` is not a valid expression: {source}"),
                    source,
                }
            }),
            other => Err(shape_error(
                self.source_file,
                path,
                format!("`when` must be a string, got {}", json_type_name(other)),
            )),
        }
    }

    /// A bare verb name as a list item: a zero-argument action such as
    /// `- stop`.
    fn parse_bare_verb(&self, raw: &str, item: &LifecyclePath) -> Result<LifecycleAction, LifecycleError> {
        let verb = raw.trim();
        let property = item.to_string();
        if verb.is_empty() {
            return Err(LifecycleError::LifecycleActionInvalidShortForm {
                source_path: self.source_file.to_path_buf(),
                property,
                raw: raw.to_string(),
                message: "action cannot be empty".to_string(),
            });
        }
        if verb.contains('(') {
            if is_set_call(verb) {
                return Err(LifecycleError::LifecycleSetNotMapping {
                    source_path: self.source_file.to_path_buf(),
                    property,
                    path: "set".to_string(),
                    actual: "call spelling".to_string(),
                });
            }
            return Err(LifecycleError::LifecycleShortFormRemoved {
                source_path: self.source_file.to_path_buf(),
                property,
                raw: raw.to_string(),
                rewrite: rewrite_to_positional(raw),
            });
        }
        if verb == "set" {
            return Err(LifecycleError::LifecycleSetNotMapping {
                source_path: self.source_file.to_path_buf(),
                property,
                path: "set".to_string(),
                actual: "bare name".to_string(),
            });
        }
        let path = item.field(verb);
        let property = path.to_string();
        if self.options.unknown_verbs.ignores(verb) {
            return Ok(ignored_action(verb, path));
        }
        if !is_known_lifecycle_verb(verb) {
            return Err(unknown_verb(self.source_file, &property, verb));
        }
        let kind = validate_positional_arity_and_build(self.signal, verb, Vec::new(), self.source_file, &property)?;
        let action = LifecycleAction {
            kind,
            no_error: false,
            path,
        };
        self.check_placement(&action)?;
        Ok(action)
    }

    /// Expand a dictionary bundle into its actions, in canonical order:
    /// `set`; the communication verbs; `shell`; side-effect and
    /// expression-function verbs by ascending ASCII name; the one directive.
    /// A bundle holds at most one directive, and the returning `prep` counts
    /// as one, so it too runs after the bundle's other verbs.
    ///
    /// `map` holds verbs plus an optional sibling `no_error`; a caller has
    /// already removed `when`. Several verbs use short form only; one verb may
    /// use long form.
    fn parse_bundle(
        &self,
        map: &serde_json::Map<String, serde_json::Value>,
        path: &LifecyclePath,
        authored: &AuthoredOrder,
    ) -> Result<Vec<LifecycleAction>, LifecycleError> {
        let sibling_no_error = match map.get("no_error") {
            None => None,
            Some(serde_json::Value::Bool(flag)) => Some(*flag),
            Some(other) => {
                return Err(shape_error(
                    self.source_file,
                    &path.field("no_error"),
                    format!("`no_error` must be a boolean, got {}", json_type_name(other)),
                ));
            }
        };
        let verbs: Vec<(&String, &serde_json::Value)> =
            map.iter().filter(|(key, _)| key.as_str() != "no_error").collect();
        if verbs.is_empty() {
            return Err(shape_error(
                self.source_file,
                path,
                "`no_error` needs an action beside it".to_string(),
            ));
        }
        let ignores = |verb: &str| self.options.unknown_verbs.ignores(verb);
        for (verb, _) in &verbs {
            if verb.as_str() != "set" && !is_known_lifecycle_verb(verb) && !ignores(verb) {
                return Err(unknown_verb(self.source_file, &path.field(verb.as_str()).to_string(), verb));
            }
        }
        if map.contains_key("say") && map.contains_key("speak") && !ignores("say") {
            return Err(LifecycleError::LifecycleSayConflict(path.to_string()));
        }
        if sibling_no_error.is_some() && verbs.len() == 1 && verbs[0].0 == "set" {
            return Err(shape_error(
                self.source_file,
                &path.field("no_error"),
                "`no_error` does not apply to `set`; a `set` failure always stops the stack"
                    .to_string(),
            ));
        }
        if verbs.iter().filter(|(verb, _)| is_lifecycle_control_verb(verb) && !ignores(verb)).count() > 1 {
            return Err(LifecycleError::LifecycleMultipleLifecycleActions {
                source_path: self.source_file.to_path_buf(),
                property: path.to_string(),
            });
        }

        let several = verbs.len() > 1;
        let mut ranked = Vec::with_capacity(verbs.len());
        for (verb, value) in verbs {
            let verb_path = path.field(verb.as_str());
            let mut action = if ignores(verb) {
                // The operand is never read, so its shape cannot fail the
                // bundle.
                ignored_action(verb, verb_path)
            } else if verb == "set" {
                self.parse_runtime_set(value, &verb_path, authored.child("set").key_order())?
            } else if let serde_json::Value::Object(params) = value {
                if several {
                    return Err(shape_error(
                        self.source_file,
                        &verb_path,
                        format!(
                            "`{verb}` uses long form, which a dictionary allows only when it holds \
                             one verb; use a list when several actions need named parameters"
                        ),
                    ));
                }
                let (action, inner_no_error) = self.parse_long_form(verb, params, &verb_path)?;
                if inner_no_error && sibling_no_error.is_some() {
                    return Err(shape_error(
                        self.source_file,
                        &verb_path.field("no_error"),
                        format!(
                            "`no_error` is set both beside `{verb}` and inside it; keep one"
                        ),
                    ));
                }
                action
            } else {
                let property = verb_path.to_string();
                let kind = parse_short_form(self.signal, verb, value, self.source_file, &property)?;
                LifecycleAction {
                    kind,
                    no_error: false,
                    path: verb_path,
                }
            };
            if let Some(flag) = sibling_no_error
                && verb != "set"
            {
                action.no_error = flag;
            }
            self.check_placement(&action)?;
            self.check_effect(&action)?;
            ranked.push((canonical_rank(verb), action));
        }
        ranked.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(ranked.into_iter().map(|(_, action)| action).collect())
    }

    /// A one-verb long form: `verb: { <params>, no_error? }`. Returns the
    /// action and whether the map carried its own `no_error`.
    fn parse_long_form(
        &self,
        verb: &str,
        map: &serde_json::Map<String, serde_json::Value>,
        path: &LifecyclePath,
    ) -> Result<(LifecycleAction, bool), LifecycleError> {
        let property = path.to_string();
        let communication = CommunicationChannel::from_verb(verb).is_some();
        let mut no_error: Option<bool> = None;
        let mut with: Option<WithOverlay> = None;
        let mut params: Vec<(String, Expr)> = Vec::new();
        for (key, value) in map {
            if key == "no_error" {
                no_error = Some(match value {
                    serde_json::Value::Bool(flag) => *flag,
                    other => {
                        return Err(LifecycleError::LifecycleActionInvalidLongForm {
                            source_path: self.source_file.to_path_buf(),
                            property,
                            action: verb.to_string(),
                            message: format!(
                                "`no_error` must be a boolean, got {}",
                                json_type_name(other)
                            ),
                            source: None,
                        });
                    }
                });
                continue;
            }
            if communication && matches!(key.as_str(), "message" | "sound" | "route") {
                let replacement = if key == "route" {
                    "remove `route`; it was never read, and messaging routes come from Claudine's \
                     configuration"
                        .to_string()
                } else {
                    format!("name the payload `text`: `{verb}: {{ text: … }}`")
                };
                return Err(LifecycleError::RemovedGrammar {
                    source_path: self.source_file.to_path_buf(),
                    path: path.field(key.as_str()).to_string(),
                    removed: format!("{verb}.{key}"),
                    replacement,
                });
            }
            // `with` is the one mapping-valued action parameter, and only the
            // directives that read a document take it.
            if key == "with" {
                if !matches!(verb, "proxy" | "retry" | "prep") {
                    return Err(LifecycleError::LifecycleWithUnsupported {
                        source_path: self.source_file.to_path_buf(),
                        property,
                        verb: verb.to_string(),
                    });
                }
                with = Some(parse_with(value, self.source_file, path, verb)?);
                continue;
            }
            // `break.code` is a literal exit code: a quoted or interpolated
            // number would pass the expression rule below.
            if verb == "break" && key == "code" && !value.is_number() {
                return Err(LifecycleError::LifecycleInvalidArgs {
                    source_path: self.source_file.to_path_buf(),
                    property,
                    action: verb.to_string(),
                    message: format!(
                        "`break.code` must be an integer from 0 to 255, got {}",
                        json_type_name(value)
                    ),
                });
            }
            // Direct YAML object literals are not parameter values; object
            // data must be passed through a whole-value `{{ ... }}` span.
            if let serde_json::Value::Object(_) = value {
                return Err(LifecycleError::LifecycleObjectDataThroughInterpolationParameter {
                    source_path: self.source_file.to_path_buf(),
                    property,
                    verb: verb.to_string(),
                    param: key.clone(),
                });
            }
            let expr = action_value_to_expr(value).map_err(|source| {
                LifecycleError::LifecycleActionInvalidLongForm {
                    source_path: self.source_file.to_path_buf(),
                    property: property.clone(),
                    action: verb.to_string(),
                    message: format!("`{property}.{key}` is not a valid value: {source}"),
                    source: Some(source),
                }
            })?;
            params.push((key.clone(), expr));
        }
        let kind = build_action_from_params(self.signal, verb, params, with, self.source_file, &property)?;
        Ok((
            LifecycleAction {
                kind,
                no_error: no_error.unwrap_or(false),
                path: path.clone(),
            },
            no_error.is_some(),
        ))
    }

    /// Parse the only authored `set` form: a literal destination map.
    /// `path` is the `set` verb's own path.
    fn parse_runtime_set(
        &self,
        value: &serde_json::Value,
        path: &LifecyclePath,
        authored_order: Option<&[String]>,
    ) -> Result<LifecycleAction, LifecycleError> {
        parse_runtime_set(value, self.source_file, path, authored_order)
    }

    /// Cancellation permits only `stop`; `skip` belongs to initialization.
    fn check_placement(&self, action: &LifecycleAction) -> Result<(), LifecycleError> {
        if let LifecycleActionKind::LifecycleControl(control) = &action.kind
            && !control.is_valid_for(self.signal)
        {
            return Err(LifecycleError::LifecycleActionPlacement {
                source_path: self.source_file.to_path_buf(),
                property: action.path.to_string(),
                action: control.verb().to_string(),
                event: event_name(self.options.registry, self.signal).to_string(),
            });
        }
        Ok(())
    }

    /// A literal `effect` name must be in the caller's catalog. A
    /// `{{ … }}` name is checked when the event resolves it.
    fn check_effect(&self, action: &LifecycleAction) -> Result<(), LifecycleError> {
        if let LifecycleActionKind::Communication(comm) = &action.kind
            && comm.channel == CommunicationChannel::Effect
            && let Expr::StringLiteral(name) = &comm.text
            && !name.contains("{{")
            && !(self.options.is_known_effect)(name)
        {
            return Err(LifecycleError::LifecycleUnknownEffect(
                action.path.to_string(),
                name.clone(),
            ));
        }
        Ok(())
    }
}

/// Where a verb falls in a bundle's canonical order.
fn canonical_rank(verb: &str) -> (u8, u8, String) {
    const COMMUNICATION: &[&str] = &[
        "stdout", "stderr", "info", "warn", "success", "message", "notify", "effect", "say",
    ];
    let communication_verb = if verb == "speak" { "say" } else { verb };
    if verb == "set" {
        (0, 0, String::new())
    } else if let Some(position) = COMMUNICATION.iter().position(|c| *c == communication_verb) {
        (1, position as u8, String::new())
    } else if verb == "shell" {
        (2, 0, String::new())
    } else if is_lifecycle_control_verb(verb) {
        (4, 0, String::new())
    } else {
        (3, 0, verb.to_string())
    }
}

fn shape_error(source_file: &Path, path: &LifecyclePath, message: String) -> LifecycleError {
    LifecycleError::LifecycleStackInvalidShape {
        source_path: source_file.to_path_buf(),
        property: path.to_string(),
        message,
    }
}

fn unknown_verb(source_file: &Path, property: &str, verb: &str) -> LifecycleError {
    let rewrite = did_you_mean_verb(verb)
        .map(|suggestion| format!("; did you mean `{suggestion}`?"))
        .unwrap_or_default();
    LifecycleError::LifecycleUnknownVerb {
        source_path: source_file.to_path_buf(),
        property: property.to_string(),
        verb: verb.to_string(),
        rewrite,
    }
}

/// The opaque action an unknown verb becomes under
/// [`UnknownVerbPolicy::Ignore`].
fn ignored_action(verb: &str, path: LifecyclePath) -> LifecycleAction {
    LifecycleAction {
        kind: LifecycleActionKind::Ignored(IgnoredAction {
            verb: verb.to_string(),
            path: path.clone(),
        }),
        no_error: false,
        path,
    }
}

fn is_set_call(raw: &str) -> bool {
    raw.trim_start().strip_prefix("set").is_some_and(|rest| rest.trim_start().starts_with('('))
}

/// Parse the only authored lifecycle `set` form: a literal destination map.
///
/// `path` is the `set` verb's own path; diagnostics name it as
/// `{parent}` plus a `set`-rooted suffix.
fn parse_runtime_set(
    value: &serde_json::Value,
    source_file: &Path,
    path: &LifecyclePath,
    authored_order: Option<&[String]>,
) -> Result<LifecycleAction, LifecycleError> {
    let owner = path.parent().to_string();
    let Some(map) = value.as_object() else {
        let actual = match value {
            serde_json::Value::Array(_) => {
                return Err(LifecycleError::LifecycleSetPositionalRemoved {
                    source_path: source_file.to_path_buf(),
                    property: owner,
                    path: "set".to_string(),
                });
            }
            serde_json::Value::String(raw) => {
                let trimmed = raw.trim();
                let spans = ExpressionFinder::find_all_plain(trimmed);
                if let Some(span) = spans.first()
                    && spans.len() == 1
                    && span.start == 0
                    && span.end == trimmed.len()
                {
                    "whole-mapping interpolation"
                } else {
                    json_type_name(value)
                }
            }
            _ => json_type_name(value),
        };
        return Err(LifecycleError::LifecycleSetNotMapping {
            source_path: source_file.to_path_buf(),
            property: owner,
            path: "set".to_string(),
            actual: actual.to_string(),
        });
    };

    let authored: IndexMap<String, serde_json::Value> = map
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let set = RuntimeSet::new_with_order(authored, authored_order).map_err(|error| match error {
        RuntimeSetError::EmptyKey => LifecycleError::LifecycleSetInvalidKey {
            source_path: source_file.to_path_buf(),
            property: owner.clone(),
            path: "set".to_string(),
            key: String::new(),
            message: "destination keys must not be empty".to_string(),
        },
        RuntimeSetError::DynamicKey(key) => LifecycleError::LifecycleSetInvalidKey {
            source_path: source_file.to_path_buf(),
            property: owner.clone(),
            path: if is_safe_path_segment(&key) {
                format!("set.{key}")
            } else {
                "set".to_string()
            },
            key,
            message: "destination keys must be literal and are never interpolated".to_string(),
        },
        RuntimeSetError::Value {
            path: value_path,
            message,
        } => LifecycleError::LifecycleActionInvalidLongForm {
            source_path: source_file.to_path_buf(),
            property: owner.clone(),
            action: "set".to_string(),
            message: format!("`{path}.{value_path}` is not a valid value: {message}"),
            source: None,
        },
    })?;

    Ok(LifecycleAction {
        kind: LifecycleActionKind::RuntimeSet(set),
        no_error: false,
        path: path.clone(),
    })
}

/// Parse the `with:` value of a `proxy`, `retry`, or `prep` long form.
///
/// Accepts only a mapping. `{}` parses to an empty overlay, which the
/// transition treats identically to an omitted `with:`. `action` is the
/// directive's own path; diagnostics hang the `with`-rooted suffix off it.
fn parse_with(
    value: &serde_json::Value,
    source_file: &Path,
    action: &LifecyclePath,
    verb: &str,
) -> Result<WithOverlay, LifecycleError> {
    let property = action.to_string();
    let map = with_mapping_shape(value, source_file, &property)?;
    let authored: IndexMap<String, serde_json::Value> = map
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    WithOverlay::new(authored).map_err(|err| match err {
        WithOverlayError::DynamicKey(key) => with_dynamic_key_error(source_file, &property, &key),
        // A value that fails the shared action-value rule is the same authoring
        // fault as a bad value on any other parameter, so it reuses that
        // diagnostic rather than adding a `with`-specific variant.
        WithOverlayError::Value {
            path: value_path,
            message,
        } => LifecycleError::LifecycleActionInvalidLongForm {
            source_path: source_file.to_path_buf(),
            property: property.clone(),
            action: verb.to_string(),
            message: format!("`{property}.with.{value_path}` is not a valid value: {message}"),
            source: None,
        },
    })
}

/// Check the shape every `with:` shares — a task's and a `proxy`, `retry`, or
/// `prep` directive's — and return its mapping.
///
/// `property` is the owner's authored path (the directive, or the task; empty
/// for the task an external `kind: task` file is). A lone whole-value span is
/// [`LifecycleError::LifecycleWithWholeMapping`], checked before the generic
/// [`LifecycleError::LifecycleWithNotMapping`] (which `null` also raises), and
/// a key carrying a `{{ … }}` or `$( … )` span is
/// [`LifecycleError::LifecycleWithDynamicKey`]. Values are not judged here.
pub fn with_mapping_shape<'a>(
    value: &'a serde_json::Value,
    source_file: &Path,
    property: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, LifecycleError> {
    let Some(map) = value.as_object() else {
        // A lone whole-value span reads as "supply the entire mapping from one
        // expression". That is a named non-goal rather than a type error, so
        // it gets its own actionable diagnostic instead of the generic one.
        if let serde_json::Value::String(raw) = value {
            let trimmed = raw.trim();
            let spans = ExpressionFinder::find_all_plain(trimmed);
            if let Some(span) = spans.first()
                && spans.len() == 1
                && span.start == 0
                && span.end == trimmed.len()
            {
                return Err(LifecycleError::LifecycleWithWholeMapping {
                    source_path: source_file.to_path_buf(),
                    property: property.to_string(),
                    path: "with".to_string(),
                    raw: raw.clone(),
                });
            }
        }
        return Err(LifecycleError::LifecycleWithNotMapping {
            source_path: source_file.to_path_buf(),
            property: property.to_string(),
            path: "with".to_string(),
            actual: json_type_name(value).to_string(),
        });
    };
    if let Some(key) = map.keys().find(|key| is_dynamic_with_key(key)) {
        return Err(with_dynamic_key_error(source_file, property, key));
    }
    Ok(map)
}

/// Whether a `with:` key carries an interpolation span. Keys name target
/// frontmatter properties and are never interpolated.
pub fn is_dynamic_with_key(key: &str) -> bool {
    !ExpressionFinder::find_all_plain(key).is_empty() || key.contains("$(")
}

fn with_dynamic_key_error(source_file: &Path, property: &str, key: &str) -> LifecycleError {
    LifecycleError::LifecycleWithDynamicKey {
        source_path: source_file.to_path_buf(),
        // A dynamic key has no honest dotted representation — `with.{{ x }}`
        // would name a property that does not exist — so the path stays
        // rooted at `with` and the key travels in its own field.
        path: if is_safe_path_segment(key) {
            format!("with.{key}")
        } else {
            "with".to_string()
        },
        property: property.to_string(),
        key: key.to_string(),
    }
}

/// Whether `key` can be appended to a dotted property path without inventing
/// a path that reads as a different property.
fn is_safe_path_segment(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

use indexmap::IndexMap;

use super::*;
use super::actions::{LifecycleConditional, for_each_action, is_lifecycle_control_verb};
use super::authored_order::AuthoredOrder;
use super::registry::{ActionType, CoreEvent, EventRegistry, KeyResolution};
use crate::markdown::compose::context::merge::json_type_name;

/// Whether an action runs a shell command: a `shell` action, or a `set` whose
/// mapping holds a whole-value `$( … )`.
pub fn runs_a_shell(action: &LifecycleAction) -> bool {
    match &action.kind {
        LifecycleActionKind::Shell(_) => true,
        LifecycleActionKind::RuntimeSet(set) => set.has_shell_values(),
        _ => false,
    }
}
