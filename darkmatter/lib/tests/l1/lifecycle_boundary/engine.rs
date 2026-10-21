//! Test-only prototype of a neutral, synchronous, request/reply lifecycle
//! engine (`2026-10-27-darkmatter-lifecycle-ownership`, Stage 1).
//!
//! The run owns every decision: which event fires next, the one terminal
//! slot, finalize accounting, catch routing, and shell permission. A host
//! reads [`Run::pending`], performs exactly that work, and answers with typed
//! facts through [`Run::reply`]. No callback can reach engine state, and no
//! event name is special here: a host's events acquire behavior only from the
//! position and role they register with.
//!
//! ## Driving a run
//!
//! ```text
//! let mut run = Run::start(&registry, DocumentInput::new("doc.md"))?;
//! loop {
//!     let request = run.pending();              // owned; holds no borrow of `run`
//!     let reply = match request.kind() {
//!         RequestKind::EvaluateEvent(event) => Reply::Event(host.evaluate(event)),
//!         RequestKind::Prepare { entry } => Reply::Prepared(host.prepare(*entry)),
//!         RequestKind::PerformOperation { .. } => Reply::Operation(host.perform()),
//!         RequestKind::Recover(recovery) => Reply::Recovery(host.recover(recovery)),
//!         RequestKind::Done(finish) => break *finish,
//!     };
//!     run.reply(request.ticket(), reply)?;
//! }
//! ```
//!
//! [`Run::transcript_lines`] then reads the ordered record of events (host
//! spelling), shell-lock transitions, selected-error changes, and the finish.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use darkmatter::markdown::compose::expression::{
    Availability, BindingView, RuntimeBinding, ScopeId, UnavailabilityReason, UnavailableBinding,
    is_reserved_namespace,
};
use serde_json::{Map, Value, json};

/// The five events Darkmatter owns. A host may rename only `Composed` and
/// `Failed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoreEvent {
    Initialize,
    Composed,
    Failed,
    Cancelled,
    Finalize,
}

impl CoreEvent {
    pub const ALL: [Self; 5] = [Self::Initialize, Self::Composed, Self::Failed, Self::Cancelled, Self::Finalize];

    pub fn darkmatter_name(self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Composed => "composed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Finalize => "finalize",
        }
    }

    /// Section 1 of the spec: `err` is unavailable until something can fail.
    fn err_availability(self) -> Availability {
        match self {
            Self::Initialize => Availability::Unavailable(UnavailabilityReason::new("lifecycle.no-error-before-composition")),
            Self::Composed => Availability::Unavailable(UnavailabilityReason::new("lifecycle.composition-succeeded")),
            Self::Failed | Self::Finalize => Availability::Available,
            Self::Cancelled => Availability::ExecutionDependent,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    BeforeComposition,
    AfterComposition,
    AfterFinalize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Notification,
    TerminalOutcome,
    IterationGate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionType {
    Expression,
    Flow,
    Communicate,
    Shell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionTypes(BTreeSet<ActionType>);

impl ActionTypes {
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
/// one; [`Registry::id`] resolves a host spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(Slot);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Slot {
    Core(CoreEvent),
    Host(usize),
}

impl EventId {
    pub const fn core(event: CoreEvent) -> Self {
        Self(Slot::Core(event))
    }
}

const INITIALIZE: EventId = EventId::core(CoreEvent::Initialize);
const COMPOSED: EventId = EventId::core(CoreEvent::Composed);
const FAILED: EventId = EventId::core(CoreEvent::Failed);
const CANCELLED: EventId = EventId::core(CoreEvent::Cancelled);
const FINALIZE: EventId = EventId::core(CoreEvent::Finalize);

/// One host event registration.
#[derive(Clone, Debug, PartialEq)]
pub struct EventDescriptor {
    pub name: String,
    pub position: Position,
    pub role: Role,
    pub globals: Arc<BindingView>,
    pub actions: ActionTypes,
}

impl EventDescriptor {
    pub fn new(name: &str, position: Position, role: Role, globals: Arc<BindingView>) -> Self {
        Self { name: name.to_string(), position, role, globals, actions: ActionTypes::all() }
    }

    #[must_use]
    pub fn with_actions(mut self, actions: ActionTypes) -> Self {
        self.actions = actions;
        self
    }
}

/// A view declaring only `err`, with `availability`.
pub fn err_view(scope: &str, availability: Availability) -> Arc<BindingView> {
    Arc::new(
        BindingView::builder(ScopeId::new(scope))
            .declare("err", "The failure the run is handling.", availability)
            .build()
            .expect("an `err`-only view is well-formed"),
    )
}

/// The terminal-outcome disposition: the alternative a successful and a
/// failed completion select, and the explicit-error downgrades allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomePair {
    pub succeeded: String,
    pub failed: String,
    pub downgrades: Vec<(String, String)>,
}

impl OutcomePair {
    pub fn new(succeeded: &str, failed: &str) -> Self {
        Self { succeeded: succeeded.to_string(), failed: failed.to_string(), downgrades: Vec::new() }
    }

    /// `from` (the success alternative or the host's spelling of `failed`)
    /// may downgrade to `to` on an explicit error.
    #[must_use]
    pub fn downgrade(mut self, from: &str, to: &str) -> Self {
        self.downgrades.push((from.to_string(), to.to_string()));
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    /// A run holds this registry; registration is fixed for a run.
    RunStarted,
    DuplicateName { name: String },
    DarkmatterEventName { name: String, event: CoreEvent },
    ReservedRoot { name: String },
    /// `initialize`, `cancelled`, and `finalize` are guarantees.
    FixedEvent { event: CoreEvent },
    SecondName { event: CoreEvent, existing: String, attempted: String },
    RoleOutOfPosition { name: String, role: Role, position: Position },
    SecondIterationGate { name: String, existing: String },
    SecondOutcomePair,
    DuplicateAlternative { name: String },
    MissingAlternative { name: String },
    DowngradeSourceNotOutcome { from: String },
    DowngradeTargetNotAlternative { from: String, to: String },
    UnpairedOutcome { name: String },
}

/// What an authored frontmatter key means to this host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyResolution {
    Event(EventId),
    /// The Darkmatter spelling of an event this host renamed: a typed parse
    /// error naming the host's spelling.
    Renamed { authored: String, replacement: String },
    /// Not a lifecycle key for this host; ordinary frontmatter data.
    Data,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Core(CoreEvent),
    Notification(Position),
    Outcome,
    Gate,
}

#[derive(Clone, Debug, PartialEq)]
struct RegistryInner {
    host: String,
    core: BTreeMap<CoreEvent, CoreSlot>,
    events: Vec<EventDescriptor>,
    pair: Option<Pair>,
}

/// A host's event registry. Mutation fails with
/// [`RegistrationError::RunStarted`] while any [`Run`] holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct Registry {
    inner: Arc<RegistryInner>,
}

impl Registry {
    /// Darkmatter's five events and nothing else: the core-only `md` shape.
    pub fn new(host: &str) -> Self {
        let core = CoreEvent::ALL
            .into_iter()
            .map(|event| {
                let name = event.darkmatter_name();
                let actions = ActionTypes::all();
                let globals = err_view(&format!("{host}.{name}"), event.err_availability());
                (event, CoreSlot { name: name.to_string(), globals, actions })
            })
            .collect();
        Self {
            inner: Arc::new(RegistryInner { host: host.to_string(), core, events: Vec::new(), pair: None }),
        }
    }

    fn edit<T>(&mut self, edit: impl FnOnce(&mut RegistryInner) -> Result<T, RegistrationError>) -> Result<T, RegistrationError> {
        edit(Arc::get_mut(&mut self.inner).ok_or(RegistrationError::RunStarted)?)
    }

    pub fn rename(&mut self, event: CoreEvent, name: &str) -> Result<(), RegistrationError> {
        self.edit(|inner| {
            if !matches!(event, CoreEvent::Composed | CoreEvent::Failed) {
                return Err(RegistrationError::FixedEvent { event });
            }
            let existing = &inner.core[&event].name;
            if existing != event.darkmatter_name() {
                return Err(RegistrationError::SecondName { event, existing: existing.clone(), attempted: name.to_string() });
            }
            inner.check_name(name)?;
            inner.core.get_mut(&event).expect("every core event has a slot").name = name.to_string();
            Ok(())
        })
    }

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
                return Err(RegistrationError::SecondIterationGate { name: descriptor.name, existing: existing.name.clone() });
            }
            inner.events.push(descriptor);
            Ok(EventId(Slot::Host(inner.events.len() - 1)))
        })
    }

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
                    .filter(|id| inner.kind(*id) == Kind::Outcome)
                    .ok_or_else(|| RegistrationError::MissingAlternative { name: name.to_string() })
            };
            let (succeeded, failed) = (alternative(&pair.succeeded)?, alternative(&pair.failed)?);
            let mut downgrades = Vec::new();
            for (from, to) in &pair.downgrades {
                let source = inner
                    .id(from)
                    .filter(|id| *id == succeeded || *id == FAILED)
                    .ok_or_else(|| RegistrationError::DowngradeSourceNotOutcome { from: from.clone() })?;
                let target = inner
                    .id(to)
                    .filter(|id| *id == succeeded || *id == failed)
                    .ok_or_else(|| RegistrationError::DowngradeTargetNotAlternative { from: from.clone(), to: to.clone() })?;
                downgrades.push((source, target));
            }
            inner.pair = Some(Pair { succeeded, failed, downgrades });
            Ok(())
        })
    }

    /// Declares the action types a Darkmatter event admits for this host.
    /// The engine's invariants still apply: `initialize` never runs a shell.
    pub fn declare_core_actions(&mut self, event: CoreEvent, actions: ActionTypes) -> Result<(), RegistrationError> {
        self.edit(|inner| {
            inner.core.get_mut(&event).expect("every core event has a slot").actions = actions;
            Ok(())
        })
    }

    /// The host's spelling of `id`.
    pub fn name(&self, id: EventId) -> &str {
        self.inner.name(id)
    }

    pub fn id(&self, name: &str) -> Option<EventId> {
        self.inner.id(name)
    }

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

    pub fn globals(&self, id: EventId) -> &Arc<BindingView> {
        match id.0 {
            Slot::Core(event) => &self.inner.core[&event].globals,
            Slot::Host(index) => &self.inner.events[index].globals,
        }
    }

    pub fn actions(&self, id: EventId) -> &ActionTypes {
        match id.0 {
            Slot::Core(event) => &self.inner.core[&event].actions,
            Slot::Host(index) => &self.inner.events[index].actions,
        }
    }

    fn kind(&self, id: EventId) -> Kind {
        self.inner.kind(id)
    }

    fn notifications(&self, position: Position) -> impl Iterator<Item = EventId> + '_ {
        (0..self.inner.events.len())
            .map(|index| EventId(Slot::Host(index)))
            .filter(move |id| self.kind(*id) == Kind::Notification(position))
    }

    fn gate(&self) -> Option<EventId> {
        (0..self.inner.events.len()).map(|index| EventId(Slot::Host(index))).find(|id| self.kind(*id) == Kind::Gate)
    }

    fn failed_alternative(&self) -> Option<EventId> {
        self.inner.pair.as_ref().map(|pair| pair.failed)
    }

    fn succeeded_alternative(&self) -> Option<EventId> {
        self.inner.pair.as_ref().map(|pair| pair.succeeded)
    }

    fn downgrade_target(&self, from: EventId) -> Option<EventId> {
        let pair = self.inner.pair.as_ref()?;
        pair.downgrades.iter().find(|(source, _)| *source == from).map(|(_, target)| *target)
    }

    fn is_terminal(&self, id: EventId) -> bool {
        id == FAILED || self.kind(id) == Kind::Outcome
    }

    /// Setup events route their failures through the catch protocol.
    fn is_setup(&self, id: EventId) -> bool {
        matches!(
            self.kind(id),
            Kind::Core(CoreEvent::Initialize | CoreEvent::Composed | CoreEvent::Failed)
                | Kind::Notification(Position::BeforeComposition | Position::AfterComposition)
        )
    }

    /// The registered failed alternative catches every setup failure (Claudine
    /// routes `initialize` errors to `failure`). Without a pair, only a
    /// failure before composition reaches `failed`; later ones go straight to
    /// `finalize` (the `md` contract in spec section 6).
    fn catch_target(&self, origin: EventId) -> Option<EventId> {
        self.failed_alternative().or_else(|| {
            matches!(self.kind(origin), Kind::Core(CoreEvent::Initialize) | Kind::Notification(Position::BeforeComposition))
                .then_some(FAILED)
        })
    }

    fn admits_recovery(&self, id: EventId, kind: RecoveryKind) -> bool {
        match self.kind(id) {
            Kind::Outcome | Kind::Core(CoreEvent::Finalize) => true,
            Kind::Core(CoreEvent::Composed) => matches!(kind, RecoveryKind::Proxy),
            _ => false,
        }
    }
}

impl RegistryInner {
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
            .or_else(|| self.events.iter().position(|event| event.name == name).map(|index| EventId(Slot::Host(index))))
    }

    fn kind(&self, id: EventId) -> Kind {
        match id.0 {
            Slot::Core(event) => Kind::Core(event),
            Slot::Host(index) => {
                let event = &self.events[index];
                match event.role {
                    Role::Notification => Kind::Notification(event.position),
                    Role::TerminalOutcome => Kind::Outcome,
                    Role::IterationGate => Kind::Gate,
                }
            }
        }
    }

    fn check_name(&self, name: &str) -> Result<(), RegistrationError> {
        if let Some(event) = CoreEvent::ALL.into_iter().find(|event| event.darkmatter_name() == name) {
            return Err(RegistrationError::DarkmatterEventName { name: name.to_string(), event });
        }
        if is_reserved_namespace(name) {
            return Err(RegistrationError::ReservedRoot { name: name.to_string() });
        }
        if self.id(name).is_some() {
            return Err(RegistrationError::DuplicateName { name: name.to_string() });
        }
        Ok(())
    }

    fn validate_for_run(&self) -> Result<(), RegistrationError> {
        let paired = |index: usize| {
            self.pair
                .as_ref()
                .is_some_and(|pair| [pair.succeeded, pair.failed].contains(&EventId(Slot::Host(index))))
        };
        match self.events.iter().enumerate().find(|(index, event)| event.role == Role::TerminalOutcome && !paired(*index)) {
            Some((_, event)) => Err(RegistrationError::UnpairedOutcome { name: event.name.clone() }),
            None => Ok(()),
        }
    }
}

/// One document for a run: its lifecycle stacks and data, split by the
/// registry when the run starts or adopts it.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentInput {
    pub label: String,
    pub frontmatter: Map<String, Value>,
}

impl DocumentInput {
    pub fn new(label: &str) -> Self {
        Self { label: label.to_string(), frontmatter: Map::new() }
    }

    #[must_use]
    pub fn with(mut self, key: &str, value: Value) -> Self {
        self.frontmatter.insert(key.to_string(), value);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartError {
    Registration(RegistrationError),
    RenamedEvent { authored: String, replacement: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunId(u64);

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

/// The identity a reply must present. Opaque: only the run issues one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    run: RunId,
    document: u32,
    attempt: u32,
    iteration: u32,
    generation: u64,
}

/// Why this attempt's preparation runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Initial,
}

/// How an event was entered, which decides whether its directives count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationMode {
    /// Terminal events and a normal `finalize` may request recovery.
    NormalRecovery,
    /// Cleanup after an expression evaluation failed; directives are ignored.
    FatalEvaluationCleanup,
    /// Cleanup after the run refused to continue: a host refusal, a refused
    /// or unsupported recovery, or a setup failure or explicit error.
    RefusalCleanup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryKind {
    Proxy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionAvailability {
    Available,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApprovalEpoch(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostError {
    pub code: String,
    pub message: String,
}

impl HostError {
    pub fn new(code: &str, message: &str) -> Self {
        Self { code: code.to_string(), message: message.to_string() }
    }
}

/// The selected error of a run and the `err` value its events read.
#[derive(Clone, Debug, PartialEq)]
pub enum LifecycleError {
    Host(HostError),
    /// A read of a global the event's view declares unavailable.
    Unavailable(Box<UnavailableBinding>),
    ShellRefused { event: String, refusal: ShellRefusal },
    /// Mirrors Claudine's `LifecycleSetupPhaseRecoveryUnsupported`.
    SetupRecoveryUnsupported { event: String, action: RecoveryKind },
}

impl LifecycleError {
    pub fn code(&self) -> &str {
        match self {
            Self::Host(error) => &error.code,
            Self::Unavailable(read) => read.reason.code(),
            Self::ShellRefused { .. } => "lifecycle.shell-refused",
            Self::SetupRecoveryUnsupported { .. } => "lifecycle.setup-recovery-unsupported",
        }
    }

    fn to_value(&self) -> Value {
        let message = match self {
            Self::Host(error) => error.message.clone(),
            Self::Unavailable(read) => read.to_string(),
            Self::ShellRefused { event, refusal } => format!("shell refused in `{event}`: {refusal:?}"),
            Self::SetupRecoveryUnsupported { event, action } => format!("{action:?} is unsupported in `{event}`"),
        };
        json!({ "code": self.code(), "msg": message })
    }
}

/// How an event may read `err`, derived from its view.
#[derive(Clone, Debug, PartialEq)]
pub enum ErrBinding {
    Value(Option<LifecycleError>),
    Unavailable { scope: ScopeId, reason: UnavailabilityReason },
    /// The view does not declare `err`; the host resolves it as data.
    Undeclared,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventRequest {
    pub event: EventId,
    pub name: String,
    pub mode: EvaluationMode,
    /// The authored stack, if the document has one for this event.
    pub stack: Option<Value>,
    /// The run state when the event fired, earlier mutations included.
    pub state: Map<String, Value>,
    pub globals: Arc<BindingView>,
    pub err: ErrBinding,
    pub actions: ActionTypes,
}

impl EventRequest {
    /// The active error, or the view's typed reason. Never a frontmatter
    /// fallback.
    pub fn read_err(&self) -> Result<Option<&LifecycleError>, LifecycleError> {
        match &self.err {
            ErrBinding::Value(error) => Ok(error.as_ref()),
            ErrBinding::Unavailable { scope, reason } => Err(LifecycleError::Unavailable(Box::new(UnavailableBinding {
                root: "err".to_string(),
                path: "err".to_string(),
                scope: scope.clone(),
                reason: reason.clone(),
                span: None,
            }))),
            ErrBinding::Undeclared => Ok(None),
        }
    }

    /// The `err` entry for a Darkmatter `EvaluationSession` over
    /// [`Self::globals`].
    pub fn runtime_bindings(&self) -> Vec<(String, RuntimeBinding<'static>)> {
        let binding = match &self.err {
            ErrBinding::Value(error) => RuntimeBinding::eager(error.as_ref().map_or(Value::Null, LifecycleError::to_value)),
            ErrBinding::Unavailable { reason, .. } => RuntimeBinding::unavailable(reason.clone()),
            ErrBinding::Undeclared => return Vec::new(),
        };
        vec![("err".to_string(), binding)]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecoveryRequest {
    pub kind: RecoveryKind,
    pub directive: Directive,
    pub from: EventId,
    pub from_name: String,
    pub launched: bool,
    pub session: SessionAvailability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finish {
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RequestKind {
    EvaluateEvent(Box<EventRequest>),
    Prepare { entry: Entry },
    PerformOperation { entry: Entry },
    Recover(RecoveryRequest),
    Done(Finish),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    ticket: Ticket,
    kind: RequestKind,
}

impl Request {
    pub fn ticket(&self) -> Ticket {
        self.ticket
    }

    pub fn kind(&self) -> &RequestKind {
        &self.kind
    }

    pub fn event(&self) -> Option<&EventRequest> {
        match &self.kind {
            RequestKind::EvaluateEvent(event) => Some(event),
            _ => None,
        }
    }

    pub fn event_name(&self) -> Option<&str> {
        self.event().map(|event| event.name.as_str())
    }
}

/// A flow directive a stack selected. Recovery directives carry their
/// evaluated arguments; the host owns budgets and targets.
#[derive(Clone, Debug, PartialEq)]
pub enum Directive {
    Error(LifecycleError),
    Proxy { target: String },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventOutcome {
    /// `set` writes, applied in order to the run state.
    pub mutations: Vec<(String, Value)>,
    /// A typed action or dispatch failure.
    pub failure: Option<LifecycleError>,
    pub evaluation_error: Option<LifecycleError>,
    pub directive: Option<Directive>,
}

impl EventOutcome {
    #[must_use]
    pub fn set(mut self, key: &str, value: Value) -> Self {
        self.mutations.push((key.to_string(), value));
        self
    }

    #[must_use]
    pub fn failure(mut self, error: LifecycleError) -> Self {
        self.failure = Some(error);
        self
    }

    #[must_use]
    pub fn evaluation_error(mut self, error: LifecycleError) -> Self {
        self.evaluation_error = Some(error);
        self
    }

    #[must_use]
    pub fn directive(mut self, directive: Directive) -> Self {
        self.directive = Some(directive);
        self
    }
}

/// Where a host-reported failure happened (the spec section 8 table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureStage {
    /// Composition, preflight, or a host preflight check.
    Preflight,
    /// Internal setup (materialization, target lifecycle parse).
    Setup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFailure {
    pub stage: FailureStage,
    /// Must match the entry of the attempt it fails.
    pub entry: Entry,
    pub error: HostError,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Preparation {
    Ready { epoch: ApprovalEpoch },
    Failed(HostFailure),
}

/// What the operation produced. A host names an outcome, never an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Succeeded,
    Failed(HostError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationReport {
    Completed { outcome: Outcome, session: SessionAvailability },
}

#[derive(Clone, Debug, PartialEq)]
pub enum RecoveryReply {
    /// A proxy handoff the host committed, with the target document.
    Adopted(DocumentInput),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    Event(EventOutcome),
    Prepared(Preparation),
    Operation(OperationReport),
    Recovery(RecoveryReply),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyError {
    ForeignRun,
    Duplicate { generation: u64 },
    Stale { generation: u64, current: u64 },
    Finished,
    WrongKind { expected: &'static str, received: &'static str },
    EntryMismatch { expected: Entry, received: Entry },
    StageNotApplicable { stage: FailureStage, request: &'static str },
    Document(StartError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellRefusal {
    /// The ticket is not the event the run is evaluating now.
    NotCurrentEvent,
    InitializeForbidsShell,
    NotAllowedInEvent { event: String },
    /// Shells unlock only on entering `composed`.
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellLock {
    Locked,
    Unlocked,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TranscriptEntry {
    Event { name: String, mode: EvaluationMode },
    Prepare(Entry),
    Operation,
    Recover(RecoveryKind),
    Adopted { document: u32 },
    Shell(ShellLock),
    Error(LifecycleError),
    Finish(Finish),
}

impl fmt::Display for TranscriptEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Event { name, .. } => write!(f, "event {name}"),
            Self::Prepare(entry) => write!(f, "prepare {}", format!("{entry:?}").to_lowercase()),
            Self::Operation => f.write_str("operation"),
            Self::Recover(kind) => write!(f, "recover {}", format!("{kind:?}").to_lowercase()),
            Self::Adopted { document } => write!(f, "adopt document {document}"),
            Self::Shell(ShellLock::Locked) => f.write_str("shell locked"),
            Self::Shell(ShellLock::Unlocked) => f.write_str("shell unlocked"),
            Self::Error(error) => write!(f, "error {}", error.code()),
            Self::Finish(finish) => write!(f, "finish {}", format!("{finish:?}").to_lowercase()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct CatchStep {
    event: EventId,
    error: Option<LifecycleError>,
    /// Re-designate an occupied `failed` slot to the catch target.
    redesignate: bool,
}

/// The catch protocol: setup failures route to the catch target, terminal
/// evaluation failures to one `finalize`; the latest evaluation error wins.
#[derive(Clone, Debug, PartialEq)]
struct Catch {
    mode: EvaluationMode,
    prior: Option<LifecycleError>,
    evaluation: Option<LifecycleError>,
    pending: Option<CatchStep>,
}

/// One lifecycle run. Hosts read [`Self::pending`] and answer with
/// [`Self::reply`]; nothing else advances it.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    registry: Registry,
    id: RunId,
    stacks: BTreeMap<EventId, Value>,
    state: Map<String, Value>,
    pending: Request,
    generation: u64,
    answered: Option<u64>,
    document: u32,
    attempt: u32,
    iteration: u32,
    entry: Entry,
    epoch: Option<ApprovalEpoch>,
    shell: ShellLock,
    terminal: Option<EventId>,
    finalized: bool,
    launched: bool,
    session: SessionAvailability,
    error: Option<LifecycleError>,
    catch: Option<Catch>,
    halted: bool,
    transcript: Vec<TranscriptEntry>,
}

/// A document's authored lifecycle stacks and its remaining data.
type SplitDocument = (BTreeMap<EventId, Value>, Map<String, Value>);

fn split_document(registry: &Registry, document: DocumentInput) -> Result<SplitDocument, StartError> {
    let mut stacks = BTreeMap::new();
    let mut state = Map::new();
    for (key, value) in document.frontmatter {
        match registry.resolve_key(&key) {
            KeyResolution::Event(id) => {
                stacks.insert(id, value);
            }
            KeyResolution::Renamed { authored, replacement } => return Err(StartError::RenamedEvent { authored, replacement }),
            KeyResolution::Data => {
                state.insert(key, value);
            }
        }
    }
    Ok((stacks, state))
}

fn reply_name(reply: &Reply) -> &'static str {
    match reply {
        Reply::Event(_) => "event",
        Reply::Prepared(_) => "prepared",
        Reply::Operation(_) => "operation",
        Reply::Recovery(_) => "recovery",
    }
}

fn request_name(kind: &RequestKind) -> &'static str {
    match kind {
        RequestKind::EvaluateEvent(_) => "evaluate-event",
        RequestKind::Prepare { .. } => "prepare",
        RequestKind::PerformOperation { .. } => "perform-operation",
        RequestKind::Recover(_) => "recover",
        RequestKind::Done(_) => "done",
    }
}

impl Run {
    pub fn start(registry: &Registry, document: DocumentInput) -> Result<Self, StartError> {
        registry.inner.validate_for_run().map_err(StartError::Registration)?;
        let (stacks, state) = split_document(registry, document)?;
        let id = RunId(NEXT_RUN.fetch_add(1, Ordering::Relaxed));
        let mut run = Self {
            registry: registry.clone(),
            id,
            stacks,
            state,
            // Replaced by the first issued request below.
            pending: Request {
                ticket: Ticket { run: id, document: 1, attempt: 1, iteration: 1, generation: 0 },
                kind: RequestKind::Done(Finish::Succeeded),
            },
            generation: 0,
            answered: None,
            document: 1,
            attempt: 1,
            iteration: 1,
            entry: Entry::Initial,
            epoch: None,
            shell: ShellLock::Locked,
            terminal: None,
            finalized: false,
            launched: false,
            session: SessionAvailability::Unavailable,
            error: None,
            catch: None,
            halted: false,
            transcript: Vec::new(),
        };
        run.fire(INITIALIZE, EvaluationMode::NormalRecovery);
        Ok(run)
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The one request the run will accept a reply for, owned.
    pub fn pending(&self) -> Request {
        self.pending.clone()
    }

    pub fn transcript(&self) -> &[TranscriptEntry] {
        &self.transcript
    }

    pub fn transcript_lines(&self) -> Vec<String> {
        self.transcript.iter().map(ToString::to_string).collect()
    }

    pub fn selected_error(&self) -> Option<&LifecycleError> {
        self.error.as_ref()
    }

    /// The host spelling of the event holding the terminal slot.
    pub fn terminal(&self) -> Option<&str> {
        self.terminal.map(|id| self.registry.name(id))
    }

    pub fn finish(&self) -> Option<Finish> {
        match self.pending.kind {
            RequestKind::Done(finish) => Some(finish),
            _ => None,
        }
    }

    pub fn state(&self) -> &Map<String, Value> {
        &self.state
    }

    /// Replies to whatever is pending; convenient for scripted hosts.
    pub fn answer(&mut self, reply: Reply) -> Result<(), ReplyError> {
        self.reply(self.pending.ticket, reply)
    }

    /// The only answer to "may this shell run now?". A host presents the
    /// ticket of the event it is evaluating; the approval epoch it gets back
    /// is the one its approval authority must check the command against.
    pub fn shell_permission(&self, ticket: &Ticket) -> Result<ApprovalEpoch, ShellRefusal> {
        let RequestKind::EvaluateEvent(request) = &self.pending.kind else {
            return Err(ShellRefusal::NotCurrentEvent);
        };
        if *ticket != self.pending.ticket {
            return Err(ShellRefusal::NotCurrentEvent);
        }
        if request.event == INITIALIZE {
            return Err(ShellRefusal::InitializeForbidsShell);
        }
        if !request.actions.contains(ActionType::Shell) {
            return Err(ShellRefusal::NotAllowedInEvent { event: request.name.clone() });
        }
        match (self.shell, self.epoch) {
            (ShellLock::Unlocked, Some(epoch)) => Ok(epoch),
            _ => Err(ShellRefusal::Locked),
        }
    }

    /// Applies `reply` to the pending request. Every error leaves the run
    /// exactly as it was.
    pub fn reply(&mut self, ticket: Ticket, reply: Reply) -> Result<(), ReplyError> {
        self.check_ticket(ticket)?;
        self.check_reply(&reply)?;
        self.answered = Some(ticket.generation);
        match (self.pending.kind.clone(), reply) {
            (RequestKind::EvaluateEvent(request), Reply::Event(outcome)) => self.on_event(&request, outcome),
            (RequestKind::Prepare { .. }, Reply::Prepared(Preparation::Ready { epoch })) => {
                self.epoch = Some(epoch);
                self.fire(COMPOSED, EvaluationMode::NormalRecovery);
            }
            (RequestKind::Prepare { .. }, Reply::Prepared(Preparation::Failed(failure))) => {
                self.set_error(LifecycleError::Host(failure.error));
                self.fire_terminal(FAILED, EvaluationMode::NormalRecovery);
            }
            (_, Reply::Operation(OperationReport::Completed { outcome, session })) => {
                self.launched = true;
                self.session = session;
                let selected = match outcome {
                    Outcome::Succeeded => self.registry.succeeded_alternative(),
                    Outcome::Failed(error) => {
                        self.set_error(LifecycleError::Host(error));
                        self.registry.failed_alternative()
                    }
                };
                match selected {
                    Some(event) => self.fire_terminal(event, EvaluationMode::NormalRecovery),
                    None => self.finish_or_finalize(EvaluationMode::NormalRecovery),
                }
            }
            (_, Reply::Recovery(RecoveryReply::Adopted(document))) => self.adopt(document),
            (kind, reply) => unreachable!("check_reply admitted {} for {}", reply_name(&reply), request_name(&kind)),
        }
        Ok(())
    }

    fn check_ticket(&self, ticket: Ticket) -> Result<(), ReplyError> {
        if ticket.run != self.id {
            return Err(ReplyError::ForeignRun);
        }
        if self.answered == Some(ticket.generation) {
            return Err(ReplyError::Duplicate { generation: ticket.generation });
        }
        if ticket != self.pending.ticket {
            return Err(ReplyError::Stale { generation: ticket.generation, current: self.pending.ticket.generation });
        }
        if self.finish().is_some() {
            return Err(ReplyError::Finished);
        }
        Ok(())
    }

    fn check_reply(&self, reply: &Reply) -> Result<(), ReplyError> {
        let entry_matches = |failure: &HostFailure, allowed: &[FailureStage], request: &'static str| {
            if failure.entry != self.entry {
                return Err(ReplyError::EntryMismatch { expected: self.entry, received: failure.entry });
            }
            if !allowed.contains(&failure.stage) {
                return Err(ReplyError::StageNotApplicable { stage: failure.stage, request });
            }
            Ok(())
        };
        match (&self.pending.kind, reply) {
            (RequestKind::EvaluateEvent(_), Reply::Event(_)) => Ok(()),
            (RequestKind::Prepare { .. }, Reply::Prepared(Preparation::Failed(failure))) => {
                entry_matches(failure, &[FailureStage::Preflight, FailureStage::Setup], "prepare")
            }
            (RequestKind::Prepare { .. }, Reply::Prepared(Preparation::Ready { .. })) => Ok(()),
            (RequestKind::PerformOperation { .. }, Reply::Operation(_)) => Ok(()),
            (RequestKind::Recover(_), Reply::Recovery(RecoveryReply::Adopted(document))) => {
                split_document(&self.registry, document.clone()).map(|_| ()).map_err(ReplyError::Document)
            }
            (kind, reply) => Err(ReplyError::WrongKind { expected: request_name(kind), received: reply_name(reply) }),
        }
    }

    fn issue(&mut self, kind: RequestKind) {
        self.generation += 1;
        let entry = match &kind {
            RequestKind::EvaluateEvent(event) => Some(TranscriptEntry::Event { name: event.name.clone(), mode: event.mode }),
            RequestKind::Prepare { entry } => Some(TranscriptEntry::Prepare(*entry)),
            RequestKind::PerformOperation { .. } => Some(TranscriptEntry::Operation),
            RequestKind::Recover(recovery) => Some(TranscriptEntry::Recover(recovery.kind)),
            RequestKind::Done(finish) => Some(TranscriptEntry::Finish(*finish)),
        };
        self.transcript.extend(entry);
        self.pending = Request {
            ticket: Ticket {
                run: self.id,
                document: self.document,
                attempt: self.attempt,
                iteration: self.iteration,
                generation: self.generation,
            },
            kind,
        };
    }

    fn fire(&mut self, event: EventId, mode: EvaluationMode) {
        if event == COMPOSED {
            self.set_shell(ShellLock::Unlocked);
        }
        if event == FINALIZE {
            self.finalized = true;
        }
        let globals = self.registry.globals(event).clone();
        let err = match globals.global("err").map(|declaration| declaration.availability()) {
            None => ErrBinding::Undeclared,
            Some(Availability::Unavailable(reason)) => {
                ErrBinding::Unavailable { scope: globals.scope().clone(), reason: reason.clone() }
            }
            Some(Availability::Available | Availability::ExecutionDependent) => ErrBinding::Value(self.error.clone()),
        };
        let request = EventRequest {
            event,
            name: self.registry.name(event).to_string(),
            mode,
            stack: self.stacks.get(&event).cloned(),
            state: self.state.clone(),
            actions: self.registry.actions(event).clone(),
            globals,
            err,
        };
        self.issue(RequestKind::EvaluateEvent(Box::new(request)));
    }

    fn fire_terminal(&mut self, event: EventId, mode: EvaluationMode) {
        self.terminal = Some(event);
        self.fire(event, mode);
    }

    fn set_shell(&mut self, lock: ShellLock) {
        if self.shell != lock {
            self.shell = lock;
            self.transcript.push(TranscriptEntry::Shell(lock));
        }
    }

    fn set_error(&mut self, error: LifecycleError) {
        if self.error.as_ref() != Some(&error) {
            self.transcript.push(TranscriptEntry::Error(error.clone()));
        }
        self.error = Some(error);
    }

    fn terminal_failed(&self) -> bool {
        self.terminal.is_some_and(|terminal| terminal == FAILED || Some(terminal) == self.registry.failed_alternative())
    }

    fn conclude(&mut self) {
        let finish = if self.error.is_some() || self.terminal_failed() {
            Finish::Failed
        } else {
            Finish::Succeeded
        };
        self.issue(RequestKind::Done(finish));
    }

    fn finish_or_finalize(&mut self, mode: EvaluationMode) {
        if self.finalized {
            self.conclude();
        } else {
            self.fire(FINALIZE, mode);
        }
    }

    fn on_event(&mut self, request: &EventRequest, outcome: EventOutcome) {
        let event = request.event;
        let EventOutcome { mutations, failure, evaluation_error, directive } = outcome;
        for (key, value) in mutations {
            self.state.insert(key, value);
        }
        if let Some(catch) = self.catch.as_mut() {
            let step = catch.pending.take().expect("a pending catch step issued this event");
            if let Some(error) = evaluation_error.clone() {
                catch.evaluation = Some(error);
            }
            catch.pending = (step.event != FINALIZE && !self.finalized).then(|| CatchStep {
                event: FINALIZE,
                error: evaluation_error.or(step.error).or_else(|| catch.prior.clone()),
                redesignate: false,
            });
            return self.drive_catch();
        }
        if request.mode == EvaluationMode::NormalRecovery
            && evaluation_error.is_none()
            && failure.is_none()
            && let Some(Directive::Error(error)) = &directive
            && let Some(target) = self.registry.downgrade_target(event)
        {
            self.set_error(error.clone());
            return self.fire_terminal(target, EvaluationMode::NormalRecovery);
        }
        let explicit = match &directive {
            Some(Directive::Error(error)) => Some(error.clone()),
            _ => None,
        };
        let setup_error = self
            .registry
            .is_setup(event)
            .then(|| evaluation_error.clone().or_else(|| failure.clone()).or(explicit))
            .flatten();
        if evaluation_error.is_some() || setup_error.is_some() {
            return self.start_catch(event, evaluation_error, setup_error);
        }
        if self.halted {
            return self.finish_or_finalize(EvaluationMode::RefusalCleanup);
        }
        if let Some(error) = failure {
            self.set_error(error);
        }
        match directive {
            Some(Directive::Error(error)) => {
                self.set_error(error);
                self.halted = true;
                self.finish_or_finalize(EvaluationMode::RefusalCleanup);
            }
            Some(directive @ Directive::Proxy { .. }) => self.request_recovery(event, directive),
            None => self.advance(event),
        }
    }

    fn start_catch(&mut self, origin: EventId, evaluation: Option<LifecycleError>, setup_error: Option<LifecycleError>) {
        let mode = if evaluation.is_some() { EvaluationMode::FatalEvaluationCleanup } else { EvaluationMode::RefusalCleanup };
        let prior = self.error.clone();
        let finalize = |error| CatchStep { event: FINALIZE, error, redesignate: false };
        let pending = if let Some(error) = setup_error {
            match self.registry.catch_target(origin) {
                Some(target) if target != origin => Some(CatchStep {
                    event: target,
                    error: Some(error),
                    redesignate: origin == FAILED && self.terminal == Some(FAILED),
                }),
                _ => (!self.finalized).then(|| finalize(Some(error))),
            }
        } else {
            let needs_finalize = self.registry.is_terminal(origin)
                || origin == CANCELLED
                || self.registry.kind(origin) == Kind::Gate;
            (needs_finalize && !self.finalized).then(|| finalize(evaluation.clone().or_else(|| prior.clone())))
        };
        self.catch = Some(Catch { mode, prior, evaluation, pending });
        self.drive_catch();
    }

    fn drive_catch(&mut self) {
        let Some(catch) = self.catch.as_mut() else { return };
        match catch.pending.clone() {
            Some(step) => {
                let mode = catch.mode;
                if let Some(error) = step.error {
                    self.set_error(error);
                }
                if self.registry.is_terminal(step.event) && (self.terminal.is_none() || step.redesignate) {
                    self.terminal = Some(step.event);
                }
                self.fire(step.event, mode);
            }
            None => {
                let evaluation = catch.evaluation.take();
                self.catch = None;
                if let Some(error) = evaluation {
                    self.set_error(error);
                }
                self.conclude();
            }
        }
    }

    fn request_recovery(&mut self, event: EventId, directive: Directive) {
        let kind = RecoveryKind::Proxy;
        if self.registry.admits_recovery(event, kind) {
            let recovery = RecoveryRequest {
                kind,
                directive,
                from: event,
                from_name: self.registry.name(event).to_string(),
                launched: self.launched,
                session: self.session,
            };
            self.issue(RequestKind::Recover(recovery));
        } else {
            self.set_error(LifecycleError::SetupRecoveryUnsupported { event: self.registry.name(event).to_string(), action: kind });
            self.halted = true;
            self.finish_or_finalize(EvaluationMode::RefusalCleanup);
        }
    }

    fn reset_attempt(&mut self) {
        self.terminal = None;
        self.finalized = false;
        self.launched = false;
        self.error = None;
        self.halted = false;
    }

    fn adopt(&mut self, document: DocumentInput) {
        let (stacks, state) = split_document(&self.registry, document).expect("check_reply validated the document");
        self.stacks = stacks;
        self.state = state;
        self.document += 1;
        self.iteration = 1;
        self.attempt = 1;
        self.entry = Entry::Initial;
        self.epoch = None;
        self.session = SessionAvailability::Unavailable;
        self.reset_attempt();
        self.set_shell(ShellLock::Locked);
        self.transcript.push(TranscriptEntry::Adopted { document: self.document });
        self.fire(INITIALIZE, EvaluationMode::NormalRecovery);
    }

    /// The next step after `event` completed normally.
    fn advance(&mut self, event: EventId) {
        let next_in = |run: &Self, position: Position, after: Option<EventId>| {
            let mut events = run.registry.notifications(position);
            match after {
                None => events.next(),
                Some(after) => events.skip_while(|id| *id != after).nth(1),
            }
        };
        let (position, after) = match self.registry.kind(event) {
            Kind::Core(CoreEvent::Initialize) => (Position::BeforeComposition, None),
            Kind::Core(CoreEvent::Composed) => (Position::AfterComposition, None),
            Kind::Core(CoreEvent::Finalize) => (Position::AfterFinalize, None),
            Kind::Notification(position) => (position, Some(event)),
            Kind::Core(CoreEvent::Failed | CoreEvent::Cancelled) | Kind::Outcome => {
                return self.finish_or_finalize(EvaluationMode::NormalRecovery);
            }
            Kind::Gate => return self.conclude(),
        };
        if let Some(next) = next_in(self, position, after) {
            return self.fire(next, EvaluationMode::NormalRecovery);
        }
        match position {
            Position::BeforeComposition => self.issue(RequestKind::Prepare { entry: self.entry }),
            Position::AfterComposition => self.issue(RequestKind::PerformOperation { entry: self.entry }),
            Position::AfterFinalize => match self.registry.gate() {
                // A failed pass ends the loop before its gate.
                Some(gate) if self.error.is_none() && !self.terminal_failed() => {
                    self.fire(gate, EvaluationMode::NormalRecovery);
                }
                _ => self.conclude(),
            },
        }
    }
}
