//! In-memory control-flow experiment. No parser, effects, provider, or production API.
#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_RUN: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role { Before, Initialize, Composed, After, Blocked, Success, Failure, Finalize, Loop }

impl Role {
    pub fn setup(self) -> bool {
        matches!(self, Self::Before | Self::Initialize | Self::Composed | Self::After | Self::Blocked)
    }
    pub fn terminal(self) -> bool {
        matches!(self, Self::Blocked | Self::Success | Self::Failure)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control { Retry, Resume, Proxy, Stop, Error, Again }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub evaluation: Option<u8>,
    pub action: Option<u8>,
    pub control: Option<Control>,
}

impl Outcome {
    pub fn control(control: Control) -> Self { Self { control: Some(control), ..Self::default() } }
    pub fn raised(code: u8) -> Self { Self { evaluation: Some(code), ..Self::default() } }
    pub fn setup_error(self, role: Role) -> Option<u8> {
        role.setup().then(|| self.evaluation.or(self.action).or(
            (self.control == Some(Control::Error)).then_some(99)
        )).flatten()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatchStep { pub role: Role, pub error: Option<u8>, pub redesignate: bool }

/// Mirrors only the existing catch protocol, including its origin-control policy.
#[derive(Clone, Debug)]
pub struct Catch {
    origin_outcome: Outcome,
    finalized: bool,
    prior: Option<u8>,
    pub setup_error: Option<u8>,
    pub evaluation: Option<(Role, u8)>,
    pub pending: Option<CatchStep>,
}

impl Catch {
    pub fn new(origin: Role, slot: Option<Role>, finalized: bool, prior: Option<u8>, outcome: Outcome) -> Self {
        let setup_error = outcome.setup_error(origin);
        let pending = if setup_error.is_some() {
            Some(CatchStep { role: Role::Failure, error: setup_error.or(prior),
                redesignate: origin == Role::Blocked && slot == Some(Role::Blocked) })
        } else if !finalized && (origin.terminal() || (origin == Role::Loop && outcome.evaluation.is_some())) {
            Some(CatchStep { role: Role::Finalize, error: outcome.evaluation.or(prior), redesignate: false })
        } else { None };
        Self { origin_outcome: outcome, finalized, prior, setup_error,
            evaluation: outcome.evaluation.map(|e| (origin, e)), pending }
    }
    pub fn record(&mut self, role: Role, outcome: Outcome) -> bool {
        let Some(step) = self.pending else { return false };
        if step.role != role { return false; }
        if let Some(error) = outcome.evaluation { self.evaluation = Some((role, error)); }
        self.pending = if role == Role::Failure && !self.finalized {
            Some(CatchStep { role: Role::Finalize,
                error: outcome.evaluation.or(step.error).or(self.prior), redesignate: false })
        } else { None };
        true
    }
    pub fn control(&self) -> Option<Control> {
        if self.pending.is_some() || self.evaluation.is_some() { None } else { self.origin_outcome.control }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery { Retry { preflight: bool }, Resume, Proxy, Exhausted, NoSession, Unsupported }

pub fn recovery(control: Control, attempt: u32, ceiling: u32, session: bool, launched: bool) -> Recovery {
    if matches!(control, Control::Retry | Control::Resume) && attempt >= ceiling {
        return Recovery::Exhausted;
    }
    match control {
        Control::Retry => Recovery::Retry { preflight: !launched },
        Control::Resume if session => Recovery::Resume,
        Control::Resume => Recovery::NoSession,
        Control::Proxy => Recovery::Proxy,
        _ => Recovery::Unsupported,
    }
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub success: &'static str,
    pub failure: &'static str,
    pub finalize: &'static str,
    pub before: Option<&'static str>,
    pub after: Option<&'static str>,
    pub looping: bool,
    pub resume: bool,
}

impl Profile {
    pub fn resolve(&self, name: &str) -> Option<Role> {
        if name == "start" { return Some(Role::Composed); }
        [Role::Initialize, Role::Composed, Role::Blocked, Role::Success,
            Role::Failure, Role::Finalize].into_iter().find(|role| self.name(*role) == name)
    }
    pub fn name(&self, role: Role) -> &'static str {
        match role {
            Role::Before => self.before.expect("registered before event"),
            Role::After => self.after.expect("registered after event"),
            Role::Initialize => "initialize", Role::Composed => "composed", Role::Blocked => "blocked",
            Role::Success => self.success, Role::Failure => self.failure,
            Role::Finalize => self.finalize, Role::Loop => "loop",
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        let mut names = vec!["initialize", "composed", "start", "blocked", self.success, self.failure, self.finalize];
        names.extend(self.before);
        names.extend(self.after);
        if self.looping { names.push("loop"); }
        names.sort_unstable();
        if names.windows(2).any(|w| w[0] == w[1]) { Err("event identity collision") } else { Ok(()) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry { Initial, Retry, Resume }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request { Event(Role), Prepare(Entry), Work, Recover(Control), Done }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket { run: usize, generation: u64, pub request: Request }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkResult { Success, Failed(u8), Blocked(u8) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    Event(Outcome), Prepared(Result<(), u8>), Worked { result: WorkResult, session: bool },
    Recovered(Result<(), u8>),
}

/// The host reports facts; it cannot set phase, authorization, or finalize state.
#[derive(Debug)]
pub struct Engine {
    profile: Profile,
    pending: Ticket,
    error: Option<u8>,
    shell: bool,
    terminal: Option<Role>,
    finalized: bool,
    halted: bool,
    catch: Option<Catch>,
    session: bool,
    launched: bool,
    attempt: u32,
    retry_ceiling: Option<u32>,
    resume_ceiling: Option<u32>,
    document: u32,
    iteration: u32,
}

impl Engine {
    pub fn new(profile: Profile) -> Result<Self, &'static str> {
        profile.validate()?;
        let role = if profile.before.is_some() { Role::Before } else { Role::Initialize };
        Ok(Self { profile, pending: Ticket { run: NEXT_RUN.fetch_add(1, Ordering::Relaxed),
            generation: 0, request: Request::Event(role) },
            error: None, shell: false, terminal: None, finalized: false, halted: false,
            catch: None, session: false, launched: false, attempt: 1, retry_ceiling: None,
            resume_ceiling: None, document: 1, iteration: 1 })
    }
    pub fn pending(&self) -> Ticket { self.pending }
    pub fn error(&self) -> Option<u8> { self.error }
    pub fn document(&self) -> u32 { self.document }
    pub fn iteration(&self) -> u32 { self.iteration }
    pub fn profile(&self) -> &Profile { &self.profile }
    pub fn shell_allowed(&self) -> bool { self.shell && self.pending.request != Request::Done }
    pub fn terminal(&self) -> Option<Role> { self.terminal }
    pub fn finalized(&self) -> bool { self.finalized }
    fn issue(&mut self, request: Request) {
        self.pending = Ticket { run: self.pending.run, generation: self.pending.generation + 1, request };
        if let Request::Event(role) = request {
            if role.terminal() { self.terminal = Some(role); }
            if role == Role::Finalize { self.finalized = true; }
        }
    }
    fn event(&mut self, role: Role) { self.issue(Request::Event(role)); }
    fn finish_or_finalize(&mut self) {
        if self.finalized { self.issue(Request::Done); } else { self.event(Role::Finalize); }
    }
    fn reset_attempt(&mut self) {
        self.terminal = None;
        self.finalized = false;
        self.launched = false;
        self.error = None;
        self.halted = false;
    }
    fn drive_catch(&mut self) {
        if let Some(step) = self.catch.as_ref().and_then(|c| c.pending) {
            self.error = step.error;
            self.event(step.role);
        } else {
            if let Some((_, code)) = self.catch.as_ref().and_then(|c| c.evaluation) { self.error = Some(code); }
            self.catch = None;
            self.issue(Request::Done);
        }
    }
    fn event_reply(&mut self, role: Role, outcome: Outcome) {
        if let Some(catch) = self.catch.as_mut() {
            assert!(catch.record(role, outcome));
            self.drive_catch();
            return;
        }
        // Normal terminal downgrade permits failure recovery. Setup catches do not.
        if outcome.evaluation.is_none() && outcome.action.is_none()
            && outcome.control == Some(Control::Error) && matches!(role, Role::Success | Role::Blocked)
        {
            self.error = Some(99);
            self.event(Role::Failure);
            return;
        }
        if outcome.evaluation.is_some() || outcome.setup_error(role).is_some() {
            self.catch = Some(Catch::new(role, self.terminal, self.finalized, self.error, outcome));
            self.drive_catch();
            return;
        }
        // Catch-only cleanup after a refused control cannot recover the refusal.
        if self.halted { self.issue(Request::Done); return; }
        if let Some(control @ (Control::Retry | Control::Resume | Control::Proxy)) = outcome.control {
            let ceiling = match control {
                Control::Retry => *self.retry_ceiling.get_or_insert(self.attempt.saturating_add(1)),
                Control::Resume => *self.resume_ceiling.get_or_insert(self.attempt.saturating_add(1)),
                _ => u32::MAX,
            };
            match recovery(control, self.attempt, ceiling, self.session && self.profile.resume, self.launched) {
                Recovery::Retry { .. } | Recovery::Resume | Recovery::Proxy => {
                    self.issue(Request::Recover(control)); return;
                }
                Recovery::NoSession | Recovery::Unsupported => {
                    self.error = Some(90); self.halted = true; self.finish_or_finalize(); return;
                }
                Recovery::Exhausted => {}
            }
        }
        if outcome.control == Some(Control::Error) {
            self.error = Some(99); self.halted = true; self.finish_or_finalize(); return;
        }
        match role {
            Role::Before => self.event(Role::Initialize),
            Role::Initialize => self.issue(Request::Prepare(Entry::Initial)),
            Role::Composed if self.profile.after.is_some() => self.event(Role::After),
            Role::Composed | Role::After => self.issue(Request::Work),
            Role::Blocked | Role::Success | Role::Failure => self.finish_or_finalize(),
            Role::Finalize if self.profile.looping => self.event(Role::Loop),
            Role::Finalize => self.issue(Request::Done),
            Role::Loop if outcome.control == Some(Control::Again) => {
                self.iteration += 1;
                self.attempt = 1;
                self.retry_ceiling = None;
                self.resume_ceiling = None;
                self.reset_attempt();
                self.event(Role::Composed);
            }
            Role::Loop => self.issue(Request::Done),
        }
    }
    pub fn reply(&mut self, ticket: Ticket, reply: Reply) -> Result<(), &'static str> {
        if ticket != self.pending { return Err("stale or foreign request"); }
        match (ticket.request, reply) {
            (Request::Event(role), Reply::Event(outcome)) => {
                if outcome.control == Some(Control::Stop) { return Err("stop is outside this spike"); }
                if outcome.control == Some(Control::Again) && role != Role::Loop {
                    return Err("iteration requires loop gate");
                }
                self.event_reply(role, outcome);
            }
            (Request::Prepare(_), Reply::Prepared(result)) => match result {
                Ok(()) => { self.shell = true; self.event(Role::Composed); }
                Err(code) => { self.error = Some(code); self.event(Role::Blocked); }
            },
            (Request::Work, Reply::Worked { result, session }) => {
                self.session = session;
                self.launched = !matches!(result, WorkResult::Blocked(_));
                match result {
                    WorkResult::Success => self.event(Role::Success),
                    WorkResult::Failed(code) => { self.error = Some(code); self.event(Role::Failure); }
                    WorkResult::Blocked(code) => { self.error = Some(code); self.event(Role::Blocked); }
                }
            }
            (Request::Recover(control), Reply::Recovered(result)) => match result {
                Err(code) => { self.error = Some(code); self.halted = true; self.finish_or_finalize(); }
                Ok(()) => {
                    self.reset_attempt();
                    match control {
                        Control::Proxy => {
                            self.document += 1;
                            self.iteration = 1;
                            self.attempt = 1;
                            self.retry_ceiling = None;
                            self.resume_ceiling = None;
                            self.shell = false;
                            self.session = false;
                            let role = if self.profile.before.is_some() { Role::Before } else { Role::Initialize };
                            self.event(role);
                        }
                        Control::Retry | Control::Resume => {
                            self.attempt += 1;
                            if control == Control::Retry { self.session = false; }
                            self.shell = false;
                            self.issue(Request::Prepare(if control == Control::Retry { Entry::Retry } else { Entry::Resume }));
                        }
                        _ => unreachable!(),
                    }
                }
            },
            _ => return Err("reply does not match pending request"),
        }
        Ok(())
    }
}

/// Alternative A: legal individual events, with ordering left to the host.
#[derive(Default)]
pub struct HostDriven {
    terminal: Option<Role>,
    finalized: bool,
}
impl HostDriven {
    pub fn emit(&mut self, role: Role) -> Result<(), &'static str> {
        if role.terminal() {
            if self.terminal.is_some() { return Err("terminal already selected"); }
            self.terminal = Some(role);
        }
        if role == Role::Finalize {
            if self.terminal.is_none() || self.finalized { return Err("invalid cleanup"); }
            self.finalized = true;
        }
        Ok(())
    }
    pub fn finish(self) -> Result<(), &'static str> {
        if self.terminal.is_some() && !self.finalized { Err("host omitted cleanup") } else { Ok(()) }
    }
}
