//! The slow-validation log: one event per schema validation behind a
//! document's published diagnostics.
//!
//! The router opens a record with [`collect`] around one document's
//! diagnostics refresh. The schema work inside it runs in [`part`]s: the
//! trigger-registry freshness check and lookup (`discover`), the overlay's
//! schema assembly (`match`, `resolve`, `compile`), and the diagnostics
//! validation (`validate`). Each part is a separate library recording
//! ([`ValidationTimings::record`]) absorbed into the record, so the event's
//! `total_ms` is the schema work alone; other providers and the overlay's
//! cache-key hashing run between parts and count nowhere. `unassigned_ms` is
//! the part time no stage claims.
//!
//! The whole refresh runs on the dispatch thread, which is also the thread
//! whose recorder the library writes to. Registry preparation runs on the
//! registry worker ahead of the refresh and is not part of any validation;
//! the worker logs it on its own (`trigger registries prepared`).
//!
//! A bundle the overlay serves from its per-document cache reuses compiled
//! validators without a library lookup; the event reports that as a compile
//! `hit`. At or above the `schema.slow_validation_ms` threshold the event is
//! a `warn` (`slow schema validation`), below it a `debug` (`schema
//! validation complete`). It never carries document values or schema bodies:
//! trigger names are file names.

use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

use darkmatter::markdown::schemas::{CompileCache, ValidationStage, ValidationTimings};

/// One document's recorded schema validation.
#[derive(Debug, Clone, Default)]
pub struct SchemaValidation {
    timings: ValidationTimings,
    compile_cache: CompileCache,
}

#[derive(Default)]
struct Record {
    timings: ValidationTimings,
    /// The overlay looked up the document's effective schema.
    assembled: bool,
    /// The overlay served a ready bundle from its own cache.
    reused_bundle: bool,
}

thread_local! {
    static OPEN: RefCell<Option<Record>> = const { RefCell::new(None) };
}

/// Clears the thread's record even when the collected body unwinds.
struct Close;

impl Drop for Close {
    fn drop(&mut self) {
        OPEN.with_borrow_mut(Option::take);
    }
}

/// Runs `body` with a record open on the calling thread.
///
/// ## Returns
///
/// The body's result, and the validation when the overlay assembled the
/// document's schema inside it; `None` for a document with no frontmatter
/// schema stage, or a refresh that stopped before reaching it.
pub fn collect<R>(body: impl FnOnce() -> R) -> (R, Option<SchemaValidation>) {
    OPEN.with_borrow_mut(|open| *open = Some(Record::default()));
    let close = Close;
    let result = body();
    let record = OPEN.with_borrow_mut(Option::take);
    drop(close);
    let validation = record.filter(|record| record.assembled).map(|record| {
        let compile_cache = match record.timings.compile_cache() {
            CompileCache::NotRequested if record.reused_bundle => CompileCache::Hit,
            state => state,
        };
        SchemaValidation { timings: record.timings, compile_cache }
    });
    (result, validation)
}

/// Runs `work` as part of the open record, if any; without one it only runs
/// `work`.
pub fn part<R>(work: impl FnOnce() -> R) -> R {
    if OPEN.with_borrow(Option::is_none) {
        return work();
    }
    let (result, timings) = ValidationTimings::record(work);
    OPEN.with_borrow_mut(|open| {
        if let Some(record) = open {
            record.timings.absorb(timings);
        }
    });
    result
}

/// Records that the overlay looked up the document's effective schema;
/// `reused_bundle` when it served a ready bundle from its cache.
pub fn note_assembly(reused_bundle: bool) {
    OPEN.with_borrow_mut(|open| {
        if let Some(record) = open {
            record.assembled = true;
            record.reused_bundle |= reused_bundle;
        }
    });
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

macro_rules! validation_event {
    ($level:expr, $message:literal, $validation:expr, $path:expr, $triggers:expr) => {
        tracing::event!(
            $level,
            path = $path,
            matched_triggers = $triggers,
            discover_ms = millis($validation.timings.stage(ValidationStage::Discover)),
            match_ms = millis($validation.timings.stage(ValidationStage::Match)),
            resolve_ms = millis($validation.timings.stage(ValidationStage::Resolve)),
            compile_ms = millis($validation.timings.stage(ValidationStage::Compile)),
            validate_ms = millis($validation.timings.stage(ValidationStage::Validate)),
            total_ms = millis($validation.timings.total()),
            unassigned_ms = millis($validation.timings.unassigned()),
            compile_cache = $validation.compile_cache.name(),
            $message
        )
    };
}

impl SchemaValidation {
    /// The stage breakdown.
    pub fn timings(&self) -> &ValidationTimings {
        &self.timings
    }

    /// Whether compilation hit the cache, counting a bundle the overlay
    /// reused as a hit.
    pub fn compile_cache(&self) -> CompileCache {
        self.compile_cache
    }

    /// Emits the event for the document at `path`: `warn` when the total is
    /// at or above `threshold`, so a zero threshold logs every validation at
    /// `warn`.
    pub fn emit(&self, path: &Path, threshold: Duration) {
        use tracing::Level;
        let path = path.to_string_lossy();
        let path = path.as_ref();
        let triggers = self.timings.matched_triggers().join(",");
        let triggers = triggers.as_str();
        if self.timings.total() >= threshold {
            validation_event!(Level::WARN, "slow schema validation", self, path, triggers);
        } else {
            validation_event!(Level::DEBUG, "schema validation complete", self, path, triggers);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_capture::CapturedLogs;

    fn emitted(validation: &SchemaValidation, threshold: Duration) -> Vec<String> {
        let ((), logs) = CapturedLogs::scoped(|| validation.emit(Path::new("/w/a.md"), threshold));
        logs.lines()
    }

    /// A recording of nothing has a zero total, so it sits exactly at a zero
    /// threshold and just below a one-millisecond one.
    #[test]
    fn the_threshold_is_inclusive() {
        let validation = SchemaValidation::default();

        let at = emitted(&validation, Duration::ZERO);
        assert_eq!(at.len(), 1, "{at:?}");
        assert!(at[0].starts_with(" WARN ") && at[0].contains("slow schema validation"), "{at:?}");

        let below = emitted(&validation, Duration::from_millis(1));
        assert_eq!(below.len(), 1, "{below:?}");
        assert!(below[0].starts_with("DEBUG ") && below[0].contains("schema validation complete"), "{below:?}");
    }

    #[test]
    fn part_outside_a_record_only_runs_the_work() {
        assert_eq!(part(|| 7), 7);
        note_assembly(true);
        let ((), validation) = collect(|| ());
        assert!(validation.is_none(), "nothing assembled inside the record");
    }

    #[test]
    fn a_reused_bundle_without_a_lookup_is_a_hit() {
        let ((), validation) = collect(|| {
            part(|| ValidationStage::Discover.time(|| ()));
            note_assembly(true);
        });
        let validation = validation.expect("assembled");
        assert_eq!(validation.compile_cache(), CompileCache::Hit);

        let ((), validation) = collect(|| note_assembly(false));
        assert_eq!(validation.expect("assembled").compile_cache(), CompileCache::NotRequested);
    }

    #[test]
    fn the_record_closes_when_the_body_unwinds() {
        let unwound = std::panic::catch_unwind(|| collect(|| panic!("provider failure")));
        assert!(unwound.is_err());
        assert!(OPEN.with_borrow(Option::is_none));
    }
}
