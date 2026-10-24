//! Stage timings for one schema validation.
//!
//! A [`ValidationTimings`] splits the work of one request into five
//! non-overlapping [`ValidationStage`]s, measured with [`Instant`]. The
//! library marks its own stages; a caller only opens a recording, either
//! with [`DarkmatterSchemas::validate_timed`] or around any work with
//! [`ValidationTimings::record`].
//!
//! Time is attributed to the innermost open stage only: a validator compiled
//! while resolving a schema counts toward `compile`, and `resolve` excludes
//! it. Work no stage claims (reading the frontmatter, building the position
//! map) is [`ValidationTimings::unassigned`].
//!
//! Recordings hold only durations, the compile-cache state, and trigger file
//! names; never document values or schema bodies.
//!
//! [`DarkmatterSchemas::validate_timed`]: super::DarkmatterSchemas::validate_timed

use std::cell::RefCell;
use std::time::{Duration, Instant};

/// One stage of a schema validation request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationStage {
    /// Trigger discovery: scanning the schema roots, parsing and checking
    /// trigger envelopes and their match definitions, and any registry
    /// freshness check a host attributes with [`ValidationStage::time`].
    Discover,
    /// Matching trigger conditions against the document.
    Match,
    /// Resolving `$schema` and trigger payloads, lowering SimplifiedSchema to
    /// JSON Schema, and merge and phase preparation.
    Resolve,
    /// Building validators. A validator found in a cache costs nothing here.
    Compile,
    /// Checking the frontmatter instance, including its working-copy
    /// conversion.
    Validate,
}

impl ValidationStage {
    /// Every stage, in request order.
    pub const ALL: [Self; 5] = [Self::Discover, Self::Match, Self::Resolve, Self::Compile, Self::Validate];

    /// The stage's lowercase name (`discover`, `match`, ...).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Discover => "discover",
            Self::Match => "match",
            Self::Resolve => "resolve",
            Self::Compile => "compile",
            Self::Validate => "validate",
        }
    }

    /// Runs `work`, attributing its time to this stage in the calling
    /// thread's active recording. Without a recording it only runs `work`.
    ///
    /// A host uses it for stage work the library cannot see, such as its own
    /// trigger-registry freshness check (`Discover`).
    pub fn time<R>(self, work: impl FnOnce() -> R) -> R {
        let _stage = stage(self);
        work()
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// Whether a request's validators came from the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompileCache {
    /// No validator was looked up or built: the document had no schema, or
    /// the request stopped before reaching a validator.
    #[default]
    NotRequested,
    /// Validators were looked up and every one was cached.
    Hit,
    /// At least one validator was built.
    Miss,
}

impl CompileCache {
    /// The state's lowercase name (`hit`, `miss`, `not requested`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::NotRequested => "not requested",
            Self::Hit => "hit",
            Self::Miss => "miss",
        }
    }

    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::Miss, _) | (_, Self::Miss) => Self::Miss,
            (Self::Hit, _) | (_, Self::Hit) => Self::Hit,
            _ => Self::NotRequested,
        }
    }
}

/// The stage breakdown of one schema validation request.
///
/// Obtained from [`DarkmatterSchemas::validate_timed`] (validation alone) or
/// [`ValidationTimings::record`] (any span of work). Partial timings of a
/// request that failed are as complete as the work that ran.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::{ValidationStage, ValidationTimings};
///
/// let ((), timings) = ValidationTimings::record(|| ());
/// assert_eq!(timings.compile_cache().name(), "not requested");
/// assert!(timings.stage(ValidationStage::Compile).is_zero());
/// ```
///
/// ## Notes
///
/// Trigger discovery usually runs when the [`DarkmatterSchemas`] is built
/// ([`DarkmatterSchemas::with_trigger_discovery`]), not inside `validate`.
/// Record that call separately and [`absorb`](Self::absorb) it into the
/// validation's timings, or record both in one closure.
///
/// [`DarkmatterSchemas`]: super::DarkmatterSchemas
/// [`DarkmatterSchemas::validate_timed`]: super::DarkmatterSchemas::validate_timed
/// [`DarkmatterSchemas::with_trigger_discovery`]: super::DarkmatterSchemas::with_trigger_discovery
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationTimings {
    stages: [Duration; 5],
    total: Duration,
    compile_cache: CompileCache,
    matched_triggers: Vec<String>,
}

impl ValidationTimings {
    /// Runs `work` under a recording on the calling thread and returns its
    /// result with the stages the library marked inside it.
    ///
    /// Recordings nest: while an inner one is open, an outer one records
    /// nothing, so the inner span counts toward the outer's unassigned time.
    pub fn record<R>(work: impl FnOnce() -> R) -> (R, Self) {
        let recording = Recording::start();
        let result = work();
        (result, recording.finish())
    }

    /// The time attributed to `stage`.
    #[must_use]
    pub fn stage(&self, stage: ValidationStage) -> Duration {
        self.stages[stage.index()]
    }

    /// Every stage with its time, in request order.
    pub fn stages(&self) -> impl Iterator<Item = (ValidationStage, Duration)> + '_ {
        ValidationStage::ALL.into_iter().map(|stage| (stage, self.stage(stage)))
    }

    /// The whole recorded span.
    #[must_use]
    pub fn total(&self) -> Duration {
        self.total
    }

    /// The part of [`Self::total`] no stage claims.
    #[must_use]
    pub fn unassigned(&self) -> Duration {
        self.total.saturating_sub(self.stages.iter().sum())
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn compile_cache(&self) -> CompileCache {
        self.compile_cache
    }

    /// The file names of the triggers whose conditions matched, in registry
    /// order. Empty when no registry was consulted.
    #[must_use]
    pub fn matched_triggers(&self) -> &[String] {
        &self.matched_triggers
    }

    /// Folds `other`, another part of the same request recorded separately,
    /// into these timings: stages and totals add, the compile-cache states
    /// combine (a miss in either is a miss), and its matched triggers follow
    /// these.
    pub fn absorb(&mut self, other: Self) {
        for (mine, theirs) in self.stages.iter_mut().zip(other.stages) {
            *mine += theirs;
        }
        self.total += other.total;
        self.compile_cache = self.compile_cache.combine(other.compile_cache);
        self.matched_triggers.extend(other.matched_triggers);
    }
}

/// An open recording on the calling thread's stack.
struct Recording {
    depth: usize,
}

impl Recording {
    fn start() -> Self {
        let depth = RECORDERS.with_borrow_mut(|recorders| {
            recorders.push(Recorder::new());
            recorders.len()
        });
        Self { depth }
    }

    fn finish(self) -> ValidationTimings {
        let finished = RECORDERS.with_borrow_mut(|recorders| {
            let recorder = recorders.drain(self.depth - 1..).next();
            recorder.map(Recorder::into_timings)
        });
        std::mem::forget(self);
        finished.unwrap_or_default()
    }
}

impl Drop for Recording {
    /// A panic inside the recorded work unwinds through here; the recording
    /// and any inner ones are discarded.
    fn drop(&mut self) {
        RECORDERS.with_borrow_mut(|recorders| recorders.truncate(self.depth - 1));
    }
}

struct Recorder {
    /// Tells a stage guard whether the recorder at its depth is still the
    /// one that opened it.
    id: u64,
    started: Instant,
    stages: [Duration; 5],
    /// Open stages, innermost last, each with the start of its current
    /// slice; only the innermost accrues time.
    open: Vec<(ValidationStage, Instant)>,
    looked_up: bool,
    built: bool,
    matched_triggers: Vec<String>,
}

impl Recorder {
    fn new() -> Self {
        Self {
            id: NEXT_RECORDER_ID.replace(NEXT_RECORDER_ID.get().wrapping_add(1)),
            started: Instant::now(),
            stages: [Duration::ZERO; 5],
            open: Vec::new(),
            looked_up: false,
            built: false,
            matched_triggers: Vec::new(),
        }
    }

    fn into_timings(mut self) -> ValidationTimings {
        let now = Instant::now();
        // A stage still open here was left by a guard that outlives the
        // recording; its time so far still belongs to this request.
        if let Some((stage, since)) = self.open.last() {
            self.stages[stage.index()] += now - *since;
        }
        let compile_cache = match (self.built, self.looked_up) {
            (true, _) => CompileCache::Miss,
            (false, true) => CompileCache::Hit,
            (false, false) => CompileCache::NotRequested,
        };
        ValidationTimings {
            stages: self.stages,
            total: now - self.started,
            compile_cache,
            matched_triggers: self.matched_triggers,
        }
    }
}

thread_local! {
    /// One recorder per open [`Recording`]; the last one records.
    static RECORDERS: RefCell<Vec<Recorder>> = const { RefCell::new(Vec::new()) };
    static NEXT_RECORDER_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Attributes the time until the guard drops to `stage` (see the module
/// docs). Inert when no recording is open.
pub(crate) fn stage(stage: ValidationStage) -> StageGuard {
    let opened = RECORDERS.with_borrow_mut(|recorders| {
        let depth = recorders.len();
        let recorder = recorders.last_mut()?;
        let now = Instant::now();
        if let Some((outer, since)) = recorder.open.last() {
            recorder.stages[outer.index()] += now - *since;
        }
        recorder.open.push((stage, now));
        Some((depth, recorder.id))
    });
    StageGuard { opened }
}

/// Closes a stage opened by [`stage`] and resumes the enclosing one.
pub(crate) struct StageGuard {
    /// The depth and id of the recorder the stage was opened in.
    opened: Option<(usize, u64)>,
}

impl Drop for StageGuard {
    fn drop(&mut self) {
        let Some((depth, id)) = self.opened else {
            return;
        };
        RECORDERS.with_borrow_mut(|recorders| {
            // The recording that opened this stage may already be finished.
            let Some(recorder) = recorders.get_mut(depth - 1).filter(|recorder| recorder.id == id) else {
                return;
            };
            let now = Instant::now();
            if let Some((stage, since)) = recorder.open.pop() {
                recorder.stages[stage.index()] += now - since;
            }
            if let Some((_, since)) = recorder.open.last_mut() {
                *since = now;
            }
        });
    }
}

/// Opens the `compile` stage for one validator build.
pub(crate) fn compiling() -> StageGuard {
    with_recorder(|recorder| recorder.built = true);
    stage(ValidationStage::Compile)
}

/// Records a validator cache lookup, hit or miss.
pub(crate) fn looked_up_validator() {
    with_recorder(|recorder| recorder.looked_up = true);
}

/// Records a trigger whose conditions matched the document.
pub(crate) fn matched_trigger(name: impl FnOnce() -> String) {
    with_recorder(|recorder| recorder.matched_triggers.push(name()));
}

fn with_recorder(update: impl FnOnce(&mut Recorder)) {
    RECORDERS.with_borrow_mut(|recorders| {
        if let Some(recorder) = recorders.last_mut() {
            update(recorder);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn busy(at_least: Duration) {
        let start = Instant::now();
        while start.elapsed() < at_least {
            std::hint::spin_loop();
        }
    }

    #[test]
    fn nested_stage_time_is_excluded_from_its_enclosing_stage() {
        let ((), timings) = ValidationTimings::record(|| {
            let _resolve = stage(ValidationStage::Resolve);
            busy(Duration::from_micros(200));
            let _compile = compiling();
            busy(Duration::from_micros(200));
        });

        assert!(timings.stage(ValidationStage::Compile) >= Duration::from_micros(200));
        assert!(timings.stage(ValidationStage::Resolve) >= Duration::from_micros(200));
        let summed: Duration = timings.stages().map(|(_, elapsed)| elapsed).sum();
        assert!(summed <= timings.total());
        assert_eq!(timings.compile_cache(), CompileCache::Miss);
    }

    #[test]
    fn stage_outside_a_recording_records_nothing() {
        let guard = stage(ValidationStage::Resolve);
        let ((), timings) = ValidationTimings::record(|| ());
        drop(guard);

        assert!(timings.stage(ValidationStage::Resolve).is_zero());
        assert_eq!(timings.compile_cache(), CompileCache::NotRequested);
    }

    #[test]
    fn inner_recording_takes_over_from_outer() {
        let ((), outer) = ValidationTimings::record(|| {
            let ((), inner) = ValidationTimings::record(looked_up_validator);
            assert_eq!(inner.compile_cache(), CompileCache::Hit);
        });

        assert_eq!(outer.compile_cache(), CompileCache::NotRequested);
    }

    #[test]
    fn absorb_combines_cache_states() {
        let ((), mut hit) = ValidationTimings::record(looked_up_validator);
        let ((), miss) = ValidationTimings::record(|| drop(compiling()));

        hit.absorb(miss);

        assert_eq!(hit.compile_cache(), CompileCache::Miss);
    }
}
