//! Option B: Merman's native measurement (`native-fonts`), given exactly the
//! chain's font files.
//!
//! Production use is `SvgEnvironment::with_native_font_context(context)`. For
//! the instrumented runs the context is wrapped in a counting
//! `HostTextMeasurer` installed through the same public policy that
//! `with_native_font_context` builds (`TextMeasurementPolicy::host_display` on
//! all phases); the wrapper only counts and forwards. `Ok(None)` is native's
//! "missing glyph coverage" decline, after which Merman measures that request
//! with its deterministic fallback.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use merman::svg::export::{NativeFontContext, NativeFontLimits, NativeFontOptions};
use merman::svg::{
    HostMeasurementResult, HostTextMeasurementRequest, HostTextMeasurer, MeasurementProfileId, SvgEnvironment,
    TextMeasurementPhase, TextMeasurementPolicy, TextMeasurementProfileIdentity,
};
use merman::OperationControl;

pub fn load_context(files: &[PathBuf]) -> Result<NativeFontContext, String> {
    let mut options = NativeFontOptions::default().with_system_fonts(false).with_limits(NativeFontLimits {
        // The resolved families (all styles) plus the CJK/emoji faces exceed the 64 MB default.
        max_bytes: 1024 * 1024 * 1024,
        ..NativeFontLimits::default()
    });
    for file in files {
        options = options.with_font_file(file);
    }
    options.load(&OperationControl::new()).map_err(|e| e.to_string())
}

pub struct CountingHost {
    context: NativeFontContext,
    pub calls: Mutex<BTreeMap<String, usize>>,
    pub declines: AtomicUsize,
    pub decline_examples: Mutex<Vec<String>>,
    pub total: AtomicUsize,
    /// Distinct `font-family` values requested during layout.
    pub families: Mutex<std::collections::BTreeSet<String>>,
}

impl HostTextMeasurer for CountingHost {
    fn measure(&self, request: HostTextMeasurementRequest<'_>) -> HostMeasurementResult {
        self.measure_with_control(request, &OperationControl::new())
    }

    fn measure_with_control(&self, request: HostTextMeasurementRequest<'_>, control: &OperationControl) -> HostMeasurementResult {
        *self.calls.lock().expect("calls").entry(format!("{:?}", request.operation)).or_default() += 1;
        self.total.fetch_add(1, Ordering::Relaxed);
        self.families.lock().expect("families").insert(request.style.font_family.clone().unwrap_or_else(|| "(none)".into()));
        let result = self.context.measure_with_control(request, control);
        if matches!(result, Ok(None)) {
            self.declines.fetch_add(1, Ordering::Relaxed);
            let mut ex = self.decline_examples.lock().expect("ex");
            if ex.len() < 6 {
                ex.push(format!("{:?}: {}", request.operation, request.text.chars().take(40).collect::<String>()));
            }
        }
        result
    }
}

/// An instrumented environment for one render, with a fresh measurement state
/// (Merman itself forks the context per operation the same way).
pub fn counting_environment(context: &NativeFontContext, control: &OperationControl) -> (SvgEnvironment, Arc<CountingHost>) {
    let host = Arc::new(CountingHost {
        context: context.for_operation(control),
        calls: Mutex::default(),
        declines: AtomicUsize::new(0),
        decline_examples: Mutex::default(),
        total: AtomicUsize::new(0),
        families: Mutex::default(),
    });
    let identity = TextMeasurementProfileIdentity::new(MeasurementProfileId::new("native-fonts").expect("id"), context.identity())
        .expect("identity");
    let policy = TextMeasurementPolicy::host_display(identity, Arc::clone(&host) as Arc<dyn HostTextMeasurer>, TextMeasurementPhase::ALL);
    (SvgEnvironment::deterministic().with_text_measurement_policy(policy), host)
}

/// The production environment.
pub fn environment(context: &NativeFontContext) -> SvgEnvironment {
    SvgEnvironment::deterministic().with_native_font_context(context.clone()).expect("native font environment")
}
