//! The event registry: which lifecycle events a host exposes, under which
//! names, and where each one fires.
//!
//! Darkmatter owns five events ([`CoreEvent`]). A host may rename `composed`
//! and `failed`, and registers its own events with a [`Position`] and a
//! [`Role`]. Its terminal-outcome events are selected through one
//! [`OutcomePair`], never by name. The registry is fixed once a run holds it:
//! every editing method fails with [`RegistrationError::RunStarted`] while
//! another clone shares it.
//!
//! The engine's slots stay [`LifecycleSignal`]s. [`EventRegistry::signal`]
//! maps a registered event to its slot (`composed` is
//! [`LifecycleSignal::Start`], `failed` is [`LifecycleSignal::Blocked`], the
//! pair's alternatives are `Success`/`Failure`, the iteration gate is `Loop`);
//! a host notification has none.
//!
//! The descriptors are passive data: they decide which frontmatter keys are
//! lifecycle stacks ([`EventRegistry::resolve_key`],
//! [`EventRegistry::authored_event_keys`]), each event's [`BindingView`], and
//! the late globals a lifecycle shell placeholder may read
//! ([`EventRegistry::host_globals`]).
//!
//! Registration is code-only, so a [`RegistrationError`] has no authored
//! source location; the failing registration call is the location.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::lifecycle::LifecycleSignal;
//! use darkmatter::lifecycle::registry::{CoreEvent, EventRegistry, KeyResolution};
//!
//! let mut registry = EventRegistry::new("review");
//! registry.rename(CoreEvent::Composed, "start").unwrap();
//! assert_eq!(registry.name_of(LifecycleSignal::Start), Some("start"));
//! assert!(matches!(registry.resolve_key("composed"), KeyResolution::Renamed { .. }));
//! assert_eq!(EventRegistry::md().resolve_key("start"), KeyResolution::Data);
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use thiserror::Error;

use crate::markdown::compose::expression::{
    Availability, BindingView, ScopeId, UnavailabilityReason, is_reserved_namespace,
};

use super::LifecycleSignal;
use crate::markdown::schemas::catalog::GlobalCatalog;

/// The five events Darkmatter owns. A host may rename only `Composed` and
/// `Failed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoreEvent {
    /// After the shell-free frontmatter bootstrap, before composition.
    Initialize,
    /// Composition and preflight succeeded.
    Composed,
    /// Composition or preflight failed, or the host refused before its
    /// operation started.
    Failed,
    /// The run was cancelled.
    Cancelled,
    /// The last event of a lifecycle.
    Finalize,
}

impl CoreEvent {
    /// Every core event, in firing order.
    pub const ALL: [Self; 5] = [Self::Initialize, Self::Composed, Self::Failed, Self::Cancelled, Self::Finalize];

    /// The name the event has under every host that does not rename it.
    pub fn darkmatter_name(self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Composed => "composed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Finalize => "finalize",
        }
    }

    /// The engine slot the event fires through.
    pub fn signal(self) -> LifecycleSignal {
        match self {
            Self::Initialize => LifecycleSignal::Initialize,
            Self::Composed => LifecycleSignal::Start,
            Self::Failed => LifecycleSignal::Blocked,
            Self::Cancelled => LifecycleSignal::Cancelled,
            Self::Finalize => LifecycleSignal::Finalize,
        }
    }

    /// `initialize`, `cancelled`, and `finalize` are guarantees whose name a
    /// host cannot change.
    pub fn renameable(self) -> bool {
        matches!(self, Self::Composed | Self::Failed)
    }

    /// Darkmatter's `err` availability: unavailable until something can fail.
    fn err_availability(self) -> Availability {
        match self {
            Self::Initialize => {
                Availability::Unavailable(UnavailabilityReason::new("lifecycle.no-error-before-composition"))
            }
            Self::Composed => Availability::Unavailable(UnavailabilityReason::new("lifecycle.composition-succeeded")),
            Self::Failed | Self::Finalize => Availability::Available,
            Self::Cancelled => Availability::ExecutionDependent,
        }
    }
}

impl fmt::Display for CoreEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.darkmatter_name())
    }
}

/// Where a host event fires relative to Darkmatter's events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Position {
    /// After `initialize`, before composition.
    BeforeComposition,
    /// After `composed`, before `finalize`.
    AfterComposition,
    /// After `finalize`.
    AfterFinalize,
}

impl Position {
    /// The authored spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BeforeComposition => "before-composition",
            Self::AfterComposition => "after-composition",
            Self::AfterFinalize => "after-finalize",
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Position {
    type Err = DescriptorParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        [Self::BeforeComposition, Self::AfterComposition, Self::AfterFinalize]
            .into_iter()
            .find(|position| position.as_str() == value)
            .ok_or_else(|| DescriptorParseError::UnknownPosition { value: value.to_string() })
    }
}

/// What a host event does in a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// An ordinary ordered stack.
    Notification,
    /// One alternative of the run's terminal slot, selected through the
    /// [`OutcomePair`].
    TerminalOutcome,
    /// Runs after `finalize` and decides whether the run re-enters.
    IterationGate,
}

impl Role {
    /// The authored spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Notification => "notification",
            Self::TerminalOutcome => "terminal-outcome",
            Self::IterationGate => "iteration-gate",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = DescriptorParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        [Self::Notification, Self::TerminalOutcome, Self::IterationGate]
            .into_iter()
            .find(|role| role.as_str() == value)
            .ok_or_else(|| DescriptorParseError::UnknownRole { value: value.to_string() })
    }
}

/// A position or role spelling that names nothing.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum DescriptorParseError {
    #[error("unknown event position `{value}`; expected before-composition, after-composition, or after-finalize")]
    UnknownPosition { value: String },
    #[error("unknown event role `{value}`; expected notification, terminal-outcome, or iteration-gate")]
    UnknownRole { value: String },
}

/// The action types of `2026-10-21-claudine-schemas`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionType {
    /// `set`, side-effect verbs, and expression functions.
    Expression,
    /// Flow directives.
    Flow,
    /// Output, speech, and messaging.
    Communicate,
    /// `shell`.
    Shell,
}

/// The action types an event admits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionTypes(std::collections::BTreeSet<ActionType>);

impl ActionTypes {
    /// Every action type.
    pub fn all() -> Self {
        Self::of([ActionType::Expression, ActionType::Flow, ActionType::Communicate, ActionType::Shell])
    }

    pub fn of(types: impl IntoIterator<Item = ActionType>) -> Self {
        Self(types.into_iter().collect())
    }

    pub fn contains(&self, action: ActionType) -> bool {
        self.0.contains(&action)
    }
}

/// An event's identity within one registry. Opaque so a host cannot invent
/// one; [`EventRegistry::id`] resolves a host spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(Slot);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Slot {
    Core(CoreEvent),
    Host(usize),
}

impl EventId {
    /// The identity of a core event, the same in every registry.
    pub const fn core(event: CoreEvent) -> Self {
        Self(Slot::Core(event))
    }
}

/// One host event registration.
#[derive(Clone, Debug, PartialEq)]
pub struct EventDescriptor {
    /// The authored frontmatter key.
    pub name: String,
    pub position: Position,
    pub role: Role,
    /// The globals the event's stack may read, with their availability.
    pub globals: Arc<BindingView>,
    /// The action types the event admits.
    pub actions: ActionTypes,
}

impl EventDescriptor {
    /// A descriptor admitting every action type.
    pub fn new(name: &str, position: Position, role: Role, globals: Arc<BindingView>) -> Self {
        Self {
            name: name.to_string(),
            position,
            role,
            globals,
            actions: ActionTypes::all(),
        }
    }

    #[must_use]
    pub fn with_actions(mut self, actions: ActionTypes) -> Self {
        self.actions = actions;
        self
    }
}

/// A view declaring Darkmatter's generic globals: `err` with `err`'s
/// availability, and `timing`, which every event may read.
///
/// ## Panics
///
/// When `scope` or the reason code of an unavailable `err` is not a valid
/// namespaced identifier.
pub fn generic_view(scope: &str, err: Availability) -> Arc<BindingView> {
    Arc::new(
        BindingView::builder(ScopeId::new(scope))
            .declare("err", "The failure the run is handling.", err)
            .declare("timing", "Observed durations.", Availability::Available)
            .build()
            .expect("the generic view declares two unreserved roots once each"),
    )
}

/// The terminal-outcome disposition: the alternative a successful and a
/// failed completion select, and the explicit-error downgrades allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomePair {
    pub succeeded: String,
    pub failed: String,
    /// `(from, to)` spellings.
    pub downgrades: Vec<(String, String)>,
}

impl OutcomePair {
    pub fn new(succeeded: &str, failed: &str) -> Self {
        Self {
            succeeded: succeeded.to_string(),
            failed: failed.to_string(),
            downgrades: Vec::new(),
        }
    }

    /// `from` (the success alternative or the host's spelling of `failed`)
    /// may downgrade to `to` on an explicit error.
    #[must_use]
    pub fn downgrade(mut self, from: &str, to: &str) -> Self {
        self.downgrades.push((from.to_string(), to.to_string()));
        self
    }
}

/// Why a registration was refused.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RegistrationError {
    /// A run holds this registry; registration is fixed for a run.
    #[error("the event registry is fixed while a run holds it")]
    RunStarted,
    #[error("event `{name}` is already registered")]
    DuplicateName { name: String },
    #[error("`{name}` is the name of Darkmatter's `{event}` event")]
    DarkmatterEventName { name: String, event: CoreEvent },
    #[error("`{name}` is a reserved root")]
    ReservedRoot { name: String },
    /// `initialize`, `cancelled`, and `finalize` are guarantees.
    #[error("`{event}` cannot be renamed")]
    FixedEvent { event: CoreEvent },
    #[error("`{event}` is already named `{existing}`; an event takes one name, not `{attempted}` as well")]
    SecondName {
        event: CoreEvent,
        existing: String,
        attempted: String,
    },
    #[error("event `{name}` cannot be a {role} event {position}")]
    RoleOutOfPosition { name: String, role: Role, position: Position },
    #[error("event `{name}` cannot be a second iteration gate; `{existing}` is one")]
    SecondIterationGate { name: String, existing: String },
    #[error("a host registers one outcome pair")]
    SecondOutcomePair,
    #[error("an outcome pair needs two distinct alternatives, not `{name}` twice")]
    DuplicateAlternative { name: String },
    #[error("outcome alternative `{name}` is not a registered terminal-outcome event")]
    MissingAlternative { name: String },
    #[error("`{from}` is neither the success alternative nor the failed event, so it cannot downgrade")]
    DowngradeSourceNotOutcome { from: String },
    #[error("`{from}` cannot downgrade to `{to}`, which is not an alternative of the outcome pair")]
    DowngradeTargetNotAlternative { from: String, to: String },
    #[error("terminal-outcome event `{name}` belongs to no outcome pair")]
    UnpairedOutcome { name: String },
    #[error("global `{name}` is already declared")]
    DuplicateGlobal { name: String },
}

/// What an authored frontmatter key means to this host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyResolution {
    /// A lifecycle stack of this event.
    Event(EventId),
    /// The Darkmatter spelling of an event this host renamed: a typed parse
    /// error naming the host's spelling.
    Renamed {
        authored: String,
        replacement: String,
    },
    /// Not a lifecycle key for this host; ordinary frontmatter data.
    Data,
}

/// A moment of a run at which the engine fires events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RunPoint {
    /// `initialize`.
    Initialize,
    /// Before-composition notifications.
    BeforeComposition,
    /// `composed`.
    Composed,
    /// After-composition notifications.
    AfterComposition,
    /// The one terminal slot: `failed` or an outcome alternative.
    Terminal,
    /// `cancelled`.
    Cancellation,
    /// `finalize`.
    Finalize,
    /// After-finalize notifications.
    AfterFinalize,
    /// The iteration gate.
    IterationGate,
}

impl RunPoint {
    /// The role an event must hold to fire here, where the point needs one.
    fn role(self) -> Option<Role> {
        match self {
            Self::Terminal => Some(Role::TerminalOutcome),
            Self::IterationGate => Some(Role::IterationGate),
            _ => None,
        }
    }
}

/// Who asks for an event to fire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emitter {
    /// The engine, advancing the run.
    Engine,
    /// An action of the stack of this event.
    Handler(EventId),
}

/// Why an event may not fire.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum EmissionError {
    /// Only the engine fires events.
    #[error("the `{handler}` stack cannot emit `{event}`; only the engine fires events")]
    HandlerEmission { handler: String, event: String },
    /// The point needs a role the event did not register.
    #[error("`{event}` is not registered as a {role} event")]
    RoleNotRegistered { event: String, role: Role },
    #[error("`{event}` fires at {registered:?}, not at {attempted:?}")]
    OutOfPosition {
        event: String,
        attempted: RunPoint,
        registered: RunPoint,
    },
    /// The identity belongs to no event of this registry.
    #[error("the event identity belongs to another registry")]
    ForeignEvent,
}

#[derive(Clone, Debug, PartialEq)]
struct CoreSlot {
    name: String,
    globals: Arc<BindingView>,
    actions: ActionTypes,
}

#[derive(Clone, Debug, PartialEq)]
struct Pair {
    succeeded: EventId,
    failed: EventId,
    downgrades: Vec<(EventId, EventId)>,
}

#[derive(Clone, Debug, PartialEq)]
struct RegistryInner {
    host: String,
    core: BTreeMap<CoreEvent, CoreSlot>,
    events: Vec<EventDescriptor>,
    pair: Option<Pair>,
    host_globals: Vec<&'static str>,
    global_catalog: Option<&'static GlobalCatalog>,
}

/// A host's event registry. Clones share one registration; editing fails
/// with [`RegistrationError::RunStarted`] while another clone exists.
#[derive(Clone, Debug, PartialEq)]
pub struct EventRegistry {
    inner: Arc<RegistryInner>,
}

/// Darkmatter's generic late globals, readable by every host's events.
const GENERIC_GLOBALS: [&str; 2] = ["err", "timing"];

impl EventRegistry {
    /// Darkmatter's five events and nothing else, for `host`.
    pub fn new(host: &str) -> Self {
        let core = CoreEvent::ALL
            .into_iter()
            .map(|event| {
                let name = event.darkmatter_name();
                let globals = generic_view(&format!("{host}.{name}"), event.err_availability());
                // Initialization is shell-free under every host; a host's
                // declaration cannot relax it.
                let actions = if event == CoreEvent::Initialize {
                    ActionTypes::of([ActionType::Expression, ActionType::Flow, ActionType::Communicate])
                } else {
                    ActionTypes::all()
                };
                let slot = CoreSlot {
                    name: name.to_string(),
                    globals,
                    actions,
                };
                (event, slot)
            })
            .collect();
        Self {
            inner: Arc::new(RegistryInner {
                host: host.to_string(),
                core,
                events: Vec::new(),
                pair: None,
                host_globals: GENERIC_GLOBALS.to_vec(),
                global_catalog: None,
            }),
        }
    }

    /// The `md` host: core events only, no renames, and no outcome pair, so
    /// its terminal slot can only ever hold `failed`.
    pub fn md() -> Self {
        Self::new("md")
    }

    fn edit<T>(
        &mut self,
        edit: impl FnOnce(&mut RegistryInner) -> Result<T, RegistrationError>,
    ) -> Result<T, RegistrationError> {
        edit(Arc::get_mut(&mut self.inner).ok_or(RegistrationError::RunStarted)?)
    }

    /// Exposes `event` as `name`.
    ///
    /// ## Errors
    ///
    /// [`RegistrationError::FixedEvent`] for an event that cannot be renamed,
    /// [`RegistrationError::SecondName`] for one already renamed, and every
    /// name collision [`Self::register`] refuses.
    pub fn rename(&mut self, event: CoreEvent, name: &str) -> Result<(), RegistrationError> {
        self.edit(|inner| {
            if !event.renameable() {
                return Err(RegistrationError::FixedEvent { event });
            }
            let existing = &inner.core[&event].name;
            if existing != event.darkmatter_name() {
                return Err(RegistrationError::SecondName {
                    event,
                    existing: existing.clone(),
                    attempted: name.to_string(),
                });
            }
            inner.check_name(name)?;
            inner.core_mut(event).name = name.to_string();
            Ok(())
        })
    }

    /// Registers a host event.
    ///
    /// ## Errors
    ///
    /// A name colliding with a Darkmatter event, a reserved root, or another
    /// registered name; a role outside its position (a terminal outcome only
    /// after composition, an iteration gate only after finalize); and a second
    /// iteration gate.
    pub fn register(&mut self, descriptor: EventDescriptor) -> Result<EventId, RegistrationError> {
        self.edit(|inner| {
            inner.check_name(&descriptor.name)?;
            let fits = match descriptor.role {
                Role::Notification => true,
                Role::TerminalOutcome => descriptor.position == Position::AfterComposition,
                Role::IterationGate => descriptor.position == Position::AfterFinalize,
            };
            if !fits {
                return Err(RegistrationError::RoleOutOfPosition {
                    name: descriptor.name,
                    role: descriptor.role,
                    position: descriptor.position,
                });
            }
            if descriptor.role == Role::IterationGate
                && let Some(existing) = inner.events.iter().find(|event| event.role == Role::IterationGate)
            {
                return Err(RegistrationError::SecondIterationGate {
                    name: descriptor.name,
                    existing: existing.name.clone(),
                });
            }
            inner.events.push(descriptor);
            Ok(EventId(Slot::Host(inner.events.len() - 1)))
        })
    }

    /// Registers the terminal-outcome pair and its downgrades.
    ///
    /// ## Errors
    ///
    /// A second pair, one alternative named twice, an alternative that is not
    /// a registered terminal-outcome event, a downgrade from anything but the
    /// success alternative or `failed`, and a downgrade to anything but an
    /// alternative.
    pub fn register_outcome_pair(&mut self, pair: OutcomePair) -> Result<(), RegistrationError> {
        self.edit(|inner| {
            if inner.pair.is_some() {
                return Err(RegistrationError::SecondOutcomePair);
            }
            if pair.succeeded == pair.failed {
                return Err(RegistrationError::DuplicateAlternative { name: pair.succeeded });
            }
            let alternative = |name: &str| {
                inner
                    .id(name)
                    .filter(|id| inner.role(*id) == Some(Role::TerminalOutcome))
                    .ok_or_else(|| RegistrationError::MissingAlternative { name: name.to_string() })
            };
            let (succeeded, failed) = (alternative(&pair.succeeded)?, alternative(&pair.failed)?);
            let mut downgrades = Vec::new();
            for (from, to) in &pair.downgrades {
                let source = inner
                    .id(from)
                    .filter(|id| *id == succeeded || *id == EventId::core(CoreEvent::Failed))
                    .ok_or_else(|| RegistrationError::DowngradeSourceNotOutcome { from: from.clone() })?;
                let target = inner
                    .id(to)
                    .filter(|id| *id == succeeded || *id == failed)
                    .ok_or_else(|| RegistrationError::DowngradeTargetNotAlternative {
                        from: from.clone(),
                        to: to.clone(),
                    })?;
                downgrades.push((source, target));
            }
            inner.pair = Some(Pair {
                succeeded,
                failed,
                downgrades,
            });
            Ok(())
        })
    }

    /// Declares the action types a Darkmatter event admits for this host.
    /// The engine's invariants still apply: `initialize` never runs a shell.
    ///
    /// ## Errors
    ///
    /// [`RegistrationError::RunStarted`].
    pub fn declare_core_actions(&mut self, event: CoreEvent, actions: ActionTypes) -> Result<(), RegistrationError> {
        self.edit(|inner| {
            let mut actions = actions;
            if event == CoreEvent::Initialize {
                actions.0.remove(&ActionType::Shell);
            }
            inner.core_mut(event).actions = actions;
            Ok(())
        })
    }

    /// Replaces the globals a Darkmatter event declares for this host.
    ///
    /// ## Errors
    ///
    /// [`RegistrationError::RunStarted`].
    pub fn declare_core_globals(
        &mut self,
        event: CoreEvent,
        globals: Arc<BindingView>,
    ) -> Result<(), RegistrationError> {
        self.edit(|inner| {
            inner.core_mut(event).globals = globals;
            Ok(())
        })
    }

    /// Adds a late global the host supplies to lifecycle evaluation, after
    /// Darkmatter's `err` and `timing`.
    ///
    /// ## Errors
    ///
    /// [`RegistrationError::ReservedRoot`] and
    /// [`RegistrationError::DuplicateGlobal`].
    pub fn declare_host_global(&mut self, name: &'static str) -> Result<(), RegistrationError> {
        self.edit(|inner| {
            if is_reserved_namespace(name) {
                return Err(RegistrationError::ReservedRoot { name: name.to_string() });
            }
            if inner.host_globals.contains(&name) {
                return Err(RegistrationError::DuplicateGlobal { name: name.to_string() });
            }
            inner.host_globals.push(name);
            Ok(())
        })
    }

    /// Registers the host catalog that types the host's globals
    /// ([`Self::global_catalog`]). Typing a global does not declare it: only
    /// [`Self::declare_host_global`] does.
    ///
    /// ## Errors
    ///
    /// [`RegistrationError::RunStarted`].
    pub fn register_global_catalog(&mut self, catalog: &'static GlobalCatalog) -> Result<(), RegistrationError> {
        self.edit(|inner| {
            inner.global_catalog = Some(catalog);
            Ok(())
        })
    }

    /// The host catalog that types the host's globals, when one is
    /// registered. Darkmatter's own globals are typed by its catalog.
    pub fn global_catalog(&self) -> Option<&'static GlobalCatalog> {
        self.inner.global_catalog
    }

    /// Checks what can only be judged once registration is complete.
    ///
    /// ## Errors
    ///
    /// [`RegistrationError::UnpairedOutcome`] for a terminal-outcome event no
    /// outcome pair selects.
    pub fn validate(&self) -> Result<(), RegistrationError> {
        let inner = &self.inner;
        let paired = |id: EventId| inner.pair.as_ref().is_some_and(|pair| pair.succeeded == id || pair.failed == id);
        match inner
            .events
            .iter()
            .enumerate()
            .find(|(index, event)| event.role == Role::TerminalOutcome && !paired(EventId(Slot::Host(*index))))
        {
            Some((_, event)) => Err(RegistrationError::UnpairedOutcome { name: event.name.clone() }),
            None => Ok(()),
        }
    }

    /// The host this registry describes.
    pub fn host(&self) -> &str {
        &self.inner.host
    }

    /// The host's spelling of `id`.
    ///
    /// ## Panics
    ///
    /// When `id` belongs to another registry.
    pub fn name(&self, id: EventId) -> &str {
        self.inner.name(id)
    }

    /// The event the host spells `name`.
    pub fn id(&self, name: &str) -> Option<EventId> {
        self.inner.id(name)
    }

    /// What the authored frontmatter key `key` is to this host.
    pub fn resolve_key(&self, key: &str) -> KeyResolution {
        if let Some(id) = self.inner.id(key) {
            return KeyResolution::Event(id);
        }
        CoreEvent::ALL
            .into_iter()
            .find(|event| event.darkmatter_name() == key)
            .map_or(KeyResolution::Data, |event| KeyResolution::Renamed {
                authored: key.to_string(),
                replacement: self.inner.core[&event].name.clone(),
            })
    }

    pub fn has_outcome_pair(&self) -> bool {
        self.inner.pair.is_some()
    }

    /// The globals `id`'s stack may read.
    pub fn globals(&self, id: EventId) -> &Arc<BindingView> {
        match id.0 {
            Slot::Core(event) => &self.inner.core[&event].globals,
            Slot::Host(index) => &self.inner.events[index].globals,
        }
    }

    /// Whether `id`'s stack may read `err`; `None` when its view does not
    /// declare `err`.
    pub fn err_availability(&self, id: EventId) -> Option<&Availability> {
        self.globals(id).global("err").map(|declaration| declaration.availability())
    }

    /// The action types `id` admits.
    pub fn actions(&self, id: EventId) -> &ActionTypes {
        match id.0 {
            Slot::Core(event) => &self.inner.core[&event].actions,
            Slot::Host(index) => &self.inner.events[index].actions,
        }
    }

    /// The core event `id` is, when it is one.
    pub fn core_event(&self, id: EventId) -> Option<CoreEvent> {
        match id.0 {
            Slot::Core(event) => Some(event),
            Slot::Host(_) => None,
        }
    }

    /// A host event's registered position; `None` for a core event.
    pub fn position(&self, id: EventId) -> Option<Position> {
        self.inner.descriptor(id).map(|descriptor| descriptor.position)
    }

    /// A host event's registered role; `None` for a core event.
    pub fn role(&self, id: EventId) -> Option<Role> {
        self.inner.role(id)
    }

    /// The notifications registered at `position`, in registration order.
    pub fn notifications(&self, position: Position) -> impl Iterator<Item = EventId> + '_ {
        self.host_events().filter(move |id| {
            self.inner
                .descriptor(*id)
                .is_some_and(|descriptor| descriptor.role == Role::Notification && descriptor.position == position)
        })
    }

    /// The iteration gate, when the host registered one.
    pub fn iteration_gate(&self) -> Option<EventId> {
        self.host_events().find(|id| self.role(*id) == Some(Role::IterationGate))
    }

    /// The alternative a successful completion selects.
    pub fn succeeded_alternative(&self) -> Option<EventId> {
        self.inner.pair.as_ref().map(|pair| pair.succeeded)
    }

    /// The alternative a failed completion selects.
    pub fn failed_alternative(&self) -> Option<EventId> {
        self.inner.pair.as_ref().map(|pair| pair.failed)
    }

    /// The alternative an explicit error in `from` downgrades to.
    pub fn downgrade_target(&self, from: EventId) -> Option<EventId> {
        let pair = self.inner.pair.as_ref()?;
        pair.downgrades.iter().find(|(source, _)| *source == from).map(|(_, target)| *target)
    }

    /// Whether `id` occupies the terminal slot when it fires: `failed` or an
    /// outcome alternative.
    pub fn is_terminal(&self, id: EventId) -> bool {
        id == EventId::core(CoreEvent::Failed) || self.role(id) == Some(Role::TerminalOutcome)
    }

    /// The engine slot `id` fires through; `None` for a host notification.
    pub fn signal(&self, id: EventId) -> Option<LifecycleSignal> {
        match id.0 {
            Slot::Core(event) => Some(event.signal()),
            Slot::Host(_) => {
                let pair = self.inner.pair.as_ref();
                if pair.is_some_and(|pair| pair.succeeded == id) {
                    Some(LifecycleSignal::Success)
                } else if pair.is_some_and(|pair| pair.failed == id) {
                    Some(LifecycleSignal::Failure)
                } else if self.role(id) == Some(Role::IterationGate) {
                    Some(LifecycleSignal::Loop)
                } else {
                    None
                }
            }
        }
    }

    /// The event that fires through `signal`; `None` when this host has none.
    pub fn id_of(&self, signal: LifecycleSignal) -> Option<EventId> {
        if let Some(event) = CoreEvent::ALL.into_iter().find(|event| event.signal() == signal) {
            return Some(EventId::core(event));
        }
        match signal {
            LifecycleSignal::Success => self.succeeded_alternative(),
            LifecycleSignal::Failure => self.failed_alternative(),
            LifecycleSignal::Loop => self.iteration_gate(),
            _ => None,
        }
    }

    /// The host's spelling of the event that fires through `signal`.
    pub fn name_of(&self, signal: LifecycleSignal) -> Option<&str> {
        self.id_of(signal).map(|id| self.name(id))
    }

    /// Every frontmatter key that holds a lifecycle stack for this host:
    /// events with an engine slot in [`LifecycleSignal::ALL`] order, then
    /// slotless notifications in registration order.
    pub fn authored_event_keys(&self) -> Vec<&str> {
        let slotted = LifecycleSignal::ALL.into_iter().filter_map(|signal| self.name_of(signal));
        let notifications = self.host_events().filter(|id| self.signal(*id).is_none()).map(|id| self.name(id));
        slotted.chain(notifications).collect()
    }

    /// The late globals a lifecycle placeholder may read that the host, not
    /// the document, supplies: Darkmatter's `err` and `timing`, then the
    /// host's own.
    pub fn host_globals(&self) -> &[&'static str] {
        &self.inner.host_globals
    }

    /// Where `id` fires.
    ///
    /// ## Panics
    ///
    /// When `id` belongs to another registry.
    pub fn point(&self, id: EventId) -> RunPoint {
        match id.0 {
            Slot::Core(CoreEvent::Initialize) => RunPoint::Initialize,
            Slot::Core(CoreEvent::Composed) => RunPoint::Composed,
            Slot::Core(CoreEvent::Failed) => RunPoint::Terminal,
            Slot::Core(CoreEvent::Cancelled) => RunPoint::Cancellation,
            Slot::Core(CoreEvent::Finalize) => RunPoint::Finalize,
            Slot::Host(index) => {
                let descriptor = &self.inner.events[index];
                match (descriptor.role, descriptor.position) {
                    (Role::TerminalOutcome, _) => RunPoint::Terminal,
                    (Role::IterationGate, _) => RunPoint::IterationGate,
                    (Role::Notification, Position::BeforeComposition) => RunPoint::BeforeComposition,
                    (Role::Notification, Position::AfterComposition) => RunPoint::AfterComposition,
                    (Role::Notification, Position::AfterFinalize) => RunPoint::AfterFinalize,
                }
            }
        }
    }

    /// Whether `emitter` may fire `event` at `point`. Only the engine fires
    /// events, and only at the point the event registered; a name alone
    /// never grants a terminal or re-entry role.
    ///
    /// ## Errors
    ///
    /// [`EmissionError::ForeignEvent`], [`EmissionError::HandlerEmission`],
    /// [`EmissionError::RoleNotRegistered`] for a point that needs a role the
    /// event lacks, and [`EmissionError::OutOfPosition`] otherwise.
    pub fn check_emission(&self, event: EventId, emitter: Emitter, point: RunPoint) -> Result<(), EmissionError> {
        if !self.inner.knows(event) {
            return Err(EmissionError::ForeignEvent);
        }
        let name = self.name(event).to_string();
        if let Emitter::Handler(handler) = emitter {
            let handler = if self.inner.knows(handler) { self.name(handler) } else { "?" };
            return Err(EmissionError::HandlerEmission {
                handler: handler.to_string(),
                event: name,
            });
        }
        let registered = self.point(event);
        if registered == point {
            return Ok(());
        }
        match point.role() {
            Some(role) if self.role(event) != Some(role) => Err(EmissionError::RoleNotRegistered { event: name, role }),
            _ => Err(EmissionError::OutOfPosition {
                event: name,
                attempted: point,
                registered,
            }),
        }
    }

    /// The host's own events, in registration order.
    pub fn host_events(&self) -> impl Iterator<Item = EventId> + '_ {
        (0..self.inner.events.len()).map(|index| EventId(Slot::Host(index)))
    }
}

impl RegistryInner {
    fn core_mut(&mut self, event: CoreEvent) -> &mut CoreSlot {
        self.core.get_mut(&event).expect("every core event has a slot")
    }

    fn knows(&self, id: EventId) -> bool {
        match id.0 {
            Slot::Core(_) => true,
            Slot::Host(index) => index < self.events.len(),
        }
    }

    fn descriptor(&self, id: EventId) -> Option<&EventDescriptor> {
        match id.0 {
            Slot::Core(_) => None,
            Slot::Host(index) => self.events.get(index),
        }
    }

    fn role(&self, id: EventId) -> Option<Role> {
        self.descriptor(id).map(|descriptor| descriptor.role)
    }

    fn name(&self, id: EventId) -> &str {
        match id.0 {
            Slot::Core(event) => &self.core[&event].name,
            Slot::Host(index) => &self.events[index].name,
        }
    }

    fn id(&self, name: &str) -> Option<EventId> {
        self.core
            .iter()
            .find(|(_, slot)| slot.name == name)
            .map(|(event, _)| EventId::core(*event))
            .or_else(|| {
                self.events
                    .iter()
                    .position(|event| event.name == name)
                    .map(|index| EventId(Slot::Host(index)))
            })
    }

    fn check_name(&self, name: &str) -> Result<(), RegistrationError> {
        if let Some(event) = CoreEvent::ALL.into_iter().find(|event| event.darkmatter_name() == name) {
            return Err(RegistrationError::DarkmatterEventName {
                name: name.to_string(),
                event,
            });
        }
        if is_reserved_namespace(name) {
            return Err(RegistrationError::ReservedRoot { name: name.to_string() });
        }
        if self.id(name).is_some() {
            return Err(RegistrationError::DuplicateName { name: name.to_string() });
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests;
