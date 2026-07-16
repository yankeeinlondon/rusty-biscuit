#[cfg_attr(test, derive(PartialEq))]
pub(crate) enum AudioPhase {
    Speak(String),
    Effect(String),
}

/// Compute the ordered audio phases for a notification.
///
/// When both speech and effect are present:
/// - `say` + `effect` → effect first, then speech
/// - `say_first` + `effect` → speech first, then effect
///
/// When only one audio output is present, it is the sole phase.
pub(crate) fn audio_phases(n: &LifecycleNotification) -> Vec<AudioPhase> {
    let speech_text = n
        .say
        .as_deref()
        .or(n.say_first.as_deref())
        .filter(|s| !s.is_empty());
    let effect_name = n.effect.as_deref().filter(|s| !s.is_empty());
    let speech_first = n.say_first.is_some();

    match (speech_text, effect_name) {
        (Some(text), Some(effect)) if speech_first => {
            vec![
                AudioPhase::Speak(text.to_string()),
                AudioPhase::Effect(effect.to_string()),
            ]
        }
        (Some(text), Some(effect)) => {
            vec![
                AudioPhase::Effect(effect.to_string()),
                AudioPhase::Speak(text.to_string()),
            ]
        }
        (Some(text), None) => vec![AudioPhase::Speak(text.to_string())],
        (None, Some(effect)) => vec![AudioPhase::Effect(effect.to_string())],
        (None, None) => vec![],
    }
}

impl LifecycleSignal {
    /// Returns the frontmatter property name for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleSignal;
    /// assert_eq!(LifecycleSignal::Start.property_name(), "start");
    /// assert_eq!(LifecycleSignal::Loop.property_name(), "loop");
    /// ```
    pub fn property_name(&self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Start => "start",
            Self::Success => "success",
            Self::Blocked => "blocked",
            Self::Failure => "failure",
            Self::Finalize => "finalize",
            Self::Loop => "loop",
        }
    }

    /// Returns the status state for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleSignal;
    /// # use biscuit_terminal::components::status::StatusState;
    /// assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
    /// assert_eq!(LifecycleSignal::Success.status_state(), StatusState::Success);
    /// assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Error);
    /// assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Error);
    /// ```
    pub fn status_state(&self) -> StatusState {
        match self {
            Self::Initialize => StatusState::Info,
            Self::Start => StatusState::Info,
            Self::Success => StatusState::Success,
            Self::Blocked | Self::Failure => StatusState::Error,
            Self::Finalize => StatusState::Info,
            Self::Loop => StatusState::Info,
        }
    }

    /// Whether this event can ever observe an error and therefore may
    /// legitimately reference the lifecycle-stack-only `err` global.
    ///
    /// Per the spec's `err` static-scan rule:
    /// - `blocked` and `failure` always carry an error.
    /// - `finalize` *optionally* carries an error (success path: no error;
    ///   failure path: error present).
    /// - `initialize`, `start`, `success`, `loop` never carry an error.
    pub const fn can_carry_error(self) -> bool {
        matches!(self, Self::Blocked | Self::Failure | Self::Finalize)
    }

    /// Whether an *unintentional* action error during this event's stack
    /// must route the run to the `failure` event.
    ///
    /// Per the spec's action error-propagation table, setup-phase events
    /// (`initialize`, `start`, `blocked`) propagate an errored action to
    /// `failure` so the agent is never invoked with a broken environment.
    /// Terminal-phase events (`success`, `failure`, `finalize`, `loop`) log
    /// the error but leave the composition outcome unchanged.
    ///
    /// This governs only unintentional action errors. The explicit `Error`
    /// lifecycle action is a deliberate author choice and follows the
    /// separate "Where valid" transition table.
    pub const fn routes_action_error_to_failure(self) -> bool {
        matches!(self, Self::Initialize | Self::Start | Self::Blocked)
    }
}

pub(super) fn normalize_empty_string(field: &mut Option<String>) {
    if let Some(s) = field {
        if s.trim().is_empty() {
            *field = None;
        }
    }
}

/// Parse a `serde_json` "unknown field" error to extract the field name
/// and the list of expected fields.
///
/// Serde's message format is:
/// `unknown field `X`, expected one of `A`, `B`, `C``
///
/// Returns `(Some("X"), vec!["A", "B", "C"])` on match. For any serde error
/// that is **not** an unknown-field error (e.g. `invalid type: map, expected
/// a sequence` when `stack:` is authored as a map instead of a list) returns
/// `(None, vec![])` — the caller renders the raw serde message rather than a
/// fabricated "Unknown property" diagnostic, since the "expected" token in a
/// type-mismatch message ("expected a sequence") is unrelated to the field
/// catalog.
pub(crate) fn tts_config_from_settings(tts: Option<&TtsSettings>) -> TtsConfig {
    let mut config = TtsConfig::new();
    let Some(settings) = tts else {
        return config;
    };

    if let Some(voice) = settings.voice.as_deref() {
        config = config.with_voice(voice);
    }
    if let Some(rate) = settings.rate {
        config = config.with_speed(SpeedLevel::Explicit(rate));
    }
    if let Some(provider) = settings.provider.as_deref() {
        if let Some(provider) = biscuit_speaks::parse_provider_name(provider) {
            config = config.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
        } else {
            warn!(
                provider,
                "Unknown TTS provider in settings; using automatic selection"
            );
        }
    }
    config
}

/// Maximum wall-clock time a single lifecycle TTS playback may block the
/// composition thread before it is abandoned.
///
/// A wedged TTS provider (a stalled network voice, a contended audio device,
/// a hung `say` subprocess) must never freeze a compose run — the danger is
/// acute *between* loop iterations, where no child wait loop is installed and
/// the Ctrl+C interrupt flag cannot reach a synchronous call. Generous enough
/// for a long sentence, short enough that a hang can't wedge the run.
const TTS_PLAYBACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Maximum wall-clock time a single lifecycle sound effect may block.
const EFFECT_PLAYBACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// Run a blocking side effect on a detached worker thread, bounding how long
/// the caller waits for it.
///
/// Returns once the work finishes or `timeout` elapses, whichever comes first.
/// On timeout the worker is detached (it may keep running harmlessly in the
/// background) and a warning is logged. This is the lifecycle analogue of the
/// wrapper's `join_with_timeout`: the only safe way to bound an arbitrary
/// blocking call (subprocess wait, audio device, network voice) from outside
/// is to stop waiting on it.
pub(super) fn run_blocking_with_timeout<F>(label: &'static str, timeout: std::time::Duration, work: F)
where
    F: FnOnce() + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        work();
        // A closed receiver (timed-out caller already moved on) is expected;
        // the send is best-effort.
        let _ = tx.send(());
    });
    match rx.recv_timeout(timeout) {
        Ok(()) => {}
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            warn!(
                label,
                ?timeout,
                "lifecycle side effect exceeded its time budget; detaching and continuing"
            );
        }
        // The worker dropped its sender without signaling (e.g. it panicked).
        // There is nothing left to wait for, and no timeout was breached.
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {}
    }
}

/// Play a sound effect synchronously (blocking), bounded by
/// [`EFFECT_PLAYBACK_TIMEOUT`].
pub(super) fn play_effect_blocking(name: &str) {
    let name = name.to_string();
    run_blocking_with_timeout("effect", EFFECT_PLAYBACK_TIMEOUT, move || {
        let Some(effect) = playa::SoundEffect::from_name(&name) else {
            warn!(%name, "Unknown sound effect in lifecycle notification");
            return;
        };
        match playa::Playa::from_bytes(effect.bytes().to_vec()) {
            Ok(player) => {
                if let Err(e) = player.play() {
                    warn!(%e, "Lifecycle sound effect playback failed");
                }
            }
            Err(e) => warn!(%e, "Failed to construct sound effect player"),
        }
    });
}

/// Speak text using the Tokio runtime, bounded by [`TTS_PLAYBACK_TIMEOUT`].
///
/// The playback runs on a detached worker thread that drives the async
/// `play()` future via the current runtime's `Handle` (cloned in before the
/// thread is spawned, since `Handle::block_on` works from any thread). This
/// both bounds the wait and avoids `block_in_place`, which would panic off a
/// runtime worker thread.
pub(super) fn say_blocking(text: &str, config: TtsConfig) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        warn!("No Tokio runtime available for lifecycle TTS");
        return;
    };
    let text = text.to_string();
    run_blocking_with_timeout("tts", TTS_PLAYBACK_TIMEOUT, move || {
        handle.block_on(async move {
            if let Err(e) = biscuit_speaks::Speak::new(text)
                .with_config(config)
                .play()
                .await
            {
                warn!(%e, "Lifecycle TTS playback failed");
            }
        });
    });
}

/// Emit a lifecycle signal with deterministic audio ordering.
///
/// Dispatches non-audio targets (stderr, message) immediately, then
/// plays audio phases in order. All errors are logged as warnings and
/// never propagated.
///
/// Prefer [`LifecycleRunGuard`] for new code — it uses this same
/// emission logic but adds mechanical state-transition enforcement.
#[deprecated(
    since = "0.1.0",
    note = "use LifecycleRunGuard instead, which enforces state transitions and emits signals via the LifecycleEmitter trait"
)]
pub fn emit_lifecycle_signal(
    config: &LifecycleConfig,
    signal: LifecycleSignal,
    ctx: &LifecycleRuntimeContext<'_>,
) {
    let Some(notification) = config.get(signal) else {
        return;
    };

    // --- Non-audio fan-out (immediate) ---

    // stderr — plain prose, no status glyph (see DefaultLifecycleEmitter::emit_stderr)
    if let Some(stderr_text) = &notification.stderr {
        let rendered = Prose::new(stderr_text).render(ctx.term);
        eprintln!("{rendered}");
    }

    // message
    if let Some(message_text) = &notification.message {
        crate::messaging::execute_resolved_message(
            message_text,
            None,
            Some(ctx.source_path),
            ctx.repo_root,
            ctx.messaging,
        );
    }
    // notify
    if let Some(notify_title) = &notification.notify {
        crate::messaging::execute_notification(notify_title, None);
    }

    // --- Audio phases (sequential, blocking, lazy TTS config) ---

    let phases = audio_phases(notification);
    let mut tts_config: Option<TtsConfig> = None;

    for phase in phases {
        match phase {
            AudioPhase::Speak(text) => {
                let config = tts_config
                    .get_or_insert_with(|| tts_config_from_settings(ctx.settings.tts.as_ref()))
                    .clone();
                say_blocking(&text, config);
            }
            AudioPhase::Effect(name) => play_effect_blocking(&name),
        }
    }
}

use super::*;
