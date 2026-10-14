//! The one background worker an invocation prepares speech on.
//!
//! [`PrewarmWorker`] is owned by the top-level command and cancels on drop;
//! [`PrewarmHandle`] is the cheap, cloneable submitter every preparation site
//! holds. Submission classifies phrases (no I/O), records each candidate's
//! [`PrewarmKey`] when it is queued, and returns without waiting. The worker
//! prepares one candidate at a time on its own thread, so provider discovery
//! and synthesis never occupy a thread of the runtime that launches the agent.

use std::collections::{BTreeSet, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use biscuit_speaks::TtsConfig;
use tokio::sync::{mpsc, watch};
use tracing::warn;

use super::super::audio::tts_config_from_settings;
use super::{Classification, Phrase, PrewarmInputs, PrewarmKey, Preparer, SpeakPreparer, classify};
use crate::events::GlobalSettings;

/// How long dropping the worker waits for its in-flight preparation to be
/// dropped. Cancellation is a future drop, not a drain: this bounds only a
/// provider that blocks inside a poll, so shutdown can never hang on it.
const CANCEL_ACKNOWLEDGEMENT: Duration = Duration::from_millis(250);

/// Why pre-warm does nothing for this invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Disabled {
    /// The command runs with `--dry-run`.
    DryRun,
    /// `config.json` sets `tts: { prewarm: false }`.
    Setting,
    /// `PLAYA_DRY_RUN` is set to one of Playa's accepted spellings.
    PlayaDryRun,
}

/// Decide whether pre-warm runs, before anything is resolved or started.
///
/// `playa_dry_run` is the raw `PLAYA_DRY_RUN` value; it accepts exactly the
/// spellings Playa and biscuit-speaks accept.
pub(crate) fn disabled_by(
    dry_run: bool,
    prewarm_setting: bool,
    playa_dry_run: Option<&str>,
) -> Option<Disabled> {
    if dry_run {
        Some(Disabled::DryRun)
    } else if !prewarm_setting {
        Some(Disabled::Setting)
    } else if matches!(playa_dry_run, Some("1" | "true" | "TRUE" | "yes")) {
        Some(Disabled::PlayaDryRun)
    } else {
        None
    }
}

/// The invocation-owned pre-warm worker.
///
/// A deferred worker starts nothing until the first submission that carries
/// phrases: that submission's settings decide the gates and the candidate
/// configuration, so a command whose documents speak nothing never starts a
/// thread or reads a setting. Dropping the worker stops accepting work and
/// cancels the preparation in flight; queued candidates are discarded, never
/// drained. Completed cache files are unaffected.
pub struct PrewarmWorker {
    slot: Option<Arc<Slot>>,
}

impl PrewarmWorker {
    /// A worker for one top-level command, started by its first submission.
    ///
    /// `dry_run` is the command's `--dry-run`; it disables pre-warm before
    /// anything is resolved.
    #[must_use]
    pub fn deferred(dry_run: bool) -> Self {
        Self::deferred_with(dry_run, production_preparer())
    }

    /// [`Self::deferred`] with the preparer injected; the gates and the
    /// candidate configuration are the production ones.
    pub(crate) fn deferred_with(dry_run: bool, preparer: Arc<dyn Preparer>) -> Self {
        // `--dry-run` is known now, so the worker is off before any site asks.
        let state = if dry_run { State::Off } else { State::Pending };
        Self {
            slot: Some(Arc::new(Slot {
                dry_run,
                preparer,
                state: Mutex::new(state),
            })),
        }
    }

    /// Start the worker now with `settings`, unless a gate disables it.
    #[cfg(test)]
    pub(crate) fn start_gated(
        settings: &GlobalSettings,
        dry_run: bool,
        preparer: Arc<dyn Preparer>,
    ) -> Self {
        let worker = Self::deferred_with(dry_run, preparer);
        worker.handle().running(Some(settings));
        worker
    }

    /// A worker that accepts nothing and starts nothing.
    #[must_use]
    pub fn disabled() -> Self {
        Self { slot: None }
    }

    /// Start a worker that prepares every candidate through `preparer`,
    /// without consulting any gate.
    #[cfg(test)]
    pub(crate) fn start_with(config: TtsConfig, preparer: Arc<dyn Preparer>) -> Self {
        let state = State::spawn(config, Arc::clone(&preparer));
        Self {
            slot: Some(Arc::new(Slot {
                dry_run: false,
                preparer,
                state: Mutex::new(state),
            })),
        }
    }

    /// A submitter for this worker; every clone shares its queue and its
    /// de-duplication.
    #[must_use]
    pub fn handle(&self) -> PrewarmHandle {
        PrewarmHandle {
            slot: self.slot.clone(),
            injected: Arc::default(),
        }
    }
}

impl Drop for PrewarmWorker {
    fn drop(&mut self) {
        let Some(slot) = &self.slot else {
            return;
        };
        let previous = std::mem::replace(&mut *slot.lock(), State::Closed);
        if let State::Running(shared, exited) = previous {
            shared.cancel.send_replace(true);
            // Waiting for the in-flight future to be dropped lets the
            // dependency clean up its synthesis child and temporary file
            // before the process exits; it never waits for synthesis itself.
            if let Some(exited) = exited {
                let _ = exited.recv_timeout(CANCEL_ACKNOWLEDGEMENT);
            }
        }
    }
}

/// The production preparer, or the `test-fixtures` recorder when a test
/// selects it.
fn production_preparer() -> Arc<dyn Preparer> {
    #[cfg(feature = "test-fixtures")]
    if let Some(recorder) = super::fixture::recording_preparer() {
        return recorder;
    }
    Arc::new(SpeakPreparer)
}

/// What a worker's handles share.
struct Slot {
    dry_run: bool,
    preparer: Arc<dyn Preparer>,
    state: Mutex<State>,
}

impl Slot {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poison| poison.into_inner())
    }
}

enum State {
    /// Deferred: the first submission with phrases decides.
    Pending,
    /// Started; the receiver reports the worker thread's exit.
    Running(Arc<Shared>, Option<std::sync::mpsc::Receiver<()>>),
    /// A gate disabled pre-warm, or the worker could not start.
    Off,
    /// The owning command dropped the worker.
    Closed,
}

impl State {
    /// Start the gated worker for `settings`.
    fn start(slot: &Slot, settings: &GlobalSettings) -> Self {
        let playa_dry_run = std::env::var("PLAYA_DRY_RUN").ok();
        match disabled_by(slot.dry_run, settings.tts_prewarm, playa_dry_run.as_deref()) {
            Some(_) => Self::Off,
            None => Self::spawn(
                tts_config_from_settings(settings.tts.as_ref()),
                Arc::clone(&slot.preparer),
            ),
        }
    }

    fn spawn(config: TtsConfig, preparer: Arc<dyn Preparer>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (cancel, cancelled) = watch::channel(false);
        let (exited_sender, exited) = std::sync::mpsc::channel();
        let shared = Arc::new(Shared {
            config: config.clone(),
            queued: Mutex::new(HashSet::new()),
            sender,
            cancel,
        });
        // The worker logs through the caller's subscriber and span, so its one
        // warning lands where the invocation's other records do.
        let dispatch = tracing::dispatcher::get_default(Clone::clone);
        let span = tracing::Span::current();
        let spawned = std::thread::Builder::new()
            .name("claudine-prewarm".into())
            .spawn(move || {
                tracing::dispatcher::with_default(&dispatch, || {
                    let _entered = span.enter();
                    match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                        Ok(runtime) => {
                            runtime.block_on(run(receiver, cancelled, config, preparer.as_ref()));
                        }
                        Err(_) => warn!("TTS pre-warm could not start; speech is synthesized when it plays"),
                    }
                });
                let _ = exited_sender.send(());
            });
        match spawned {
            Ok(_) => Self::Running(shared, Some(exited)),
            Err(_) => {
                warn!("TTS pre-warm could not start; speech is synthesized when it plays");
                Self::Off
            }
        }
    }
}

/// Submits candidates to an invocation's [`PrewarmWorker`].
///
/// Every method returns without waiting for preparation. A handle whose worker
/// is disabled or dropped accepts nothing and resolves nothing.
#[derive(Clone)]
pub struct PrewarmHandle {
    slot: Option<Arc<Slot>>,
    /// Frontmatter keys a sequence injects into every document submitted
    /// through this handle; they are late even though they read as
    /// frontmatter.
    injected: Arc<BTreeSet<String>>,
}

impl Default for PrewarmHandle {
    /// A handle with no worker: it accepts nothing.
    fn default() -> Self {
        PrewarmWorker::disabled().handle()
    }
}

impl std::fmt::Debug for PrewarmHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrewarmHandle")
            .field("enabled", &self.is_enabled())
            .field("injected", &self.injected)
            .finish()
    }
}

struct Shared {
    config: TtsConfig,
    /// Every key ever queued in this invocation, so a candidate is attempted
    /// at most once, whether it succeeded, failed, or is still in progress.
    queued: Mutex<HashSet<PrewarmKey>>,
    sender: mpsc::UnboundedSender<String>,
    cancel: watch::Sender<bool>,
}

impl PrewarmHandle {
    /// Whether submitted phrases can still reach a worker: it has started, or
    /// it is deferred and no gate has been decided yet.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        let Some(slot) = &self.slot else {
            return false;
        };
        match &*slot.lock() {
            State::Pending => true,
            State::Running(shared, _) => !*shared.cancel.borrow(),
            State::Off | State::Closed => false,
        }
    }

    /// This handle, also treating `keys` as sequence-injected for every
    /// submission made through it and its clones.
    #[must_use]
    pub fn with_injected_keys(&self, keys: impl IntoIterator<Item = String>) -> Self {
        let mut injected = (*self.injected).clone();
        injected.extend(keys);
        Self {
            slot: self.slot.clone(),
            injected: Arc::new(injected),
        }
    }

    /// Classify `phrases` against `inputs` and queue each early one, starting
    /// a deferred worker with `settings` first.
    ///
    /// Returns how many new candidates were queued.
    pub(crate) fn submit<'a>(
        &self,
        settings: &GlobalSettings,
        phrases: Vec<Phrase<'a>>,
        inputs: &PrewarmInputs,
    ) -> usize {
        if phrases.is_empty() {
            return 0;
        }
        match self.running(Some(settings)) {
            Some(shared) => self.queue(&shared, phrases, inputs),
            None => 0,
        }
    }

    /// Classify `phrases` against `inputs` and queue each early one on a
    /// started worker.
    ///
    /// Returns how many new candidates were queued. Nothing is classified when
    /// the worker is deferred, disabled, or cancelled.
    #[cfg(test)]
    pub(crate) fn submit_phrases<'a>(
        &self,
        phrases: impl IntoIterator<Item = Phrase<'a>>,
        inputs: &PrewarmInputs,
    ) -> usize {
        match self.running(None) {
            Some(shared) => self.queue(&shared, phrases, inputs),
            None => 0,
        }
    }

    /// The started, uncancelled worker, starting a deferred one when
    /// `settings` are given.
    fn running(&self, settings: Option<&GlobalSettings>) -> Option<Arc<Shared>> {
        let slot = self.slot.as_ref()?;
        let mut state = slot.lock();
        if let (State::Pending, Some(settings)) = (&*state, settings) {
            *state = State::start(slot, settings);
        }
        match &*state {
            State::Running(shared, _) if !*shared.cancel.borrow() => Some(Arc::clone(shared)),
            _ => None,
        }
    }

    fn queue<'a>(
        &self,
        shared: &Shared,
        phrases: impl IntoIterator<Item = Phrase<'a>>,
        inputs: &PrewarmInputs,
    ) -> usize {
        let scoped;
        let inputs = if self.injected.is_empty() {
            inputs
        } else {
            let mut widened = inputs.clone();
            widened
                .sequence_injected_keys
                .extend(self.injected.iter().cloned());
            scoped = widened;
            &scoped
        };
        phrases
            .into_iter()
            .filter_map(|phrase| match classify(phrase.scope, phrase.text, inputs) {
                Classification::Early(text) => Some(text),
                Classification::Skip(_) => None,
            })
            .filter(|text| shared.enqueue(text))
            .count()
    }
}

impl Shared {
    /// Queue `text` unless its key was queued before; `true` when queued.
    fn enqueue(&self, text: &str) -> bool {
        let key = PrewarmKey::new(text, &self.config);
        let mut queued = self.queued.lock().unwrap_or_else(|poison| poison.into_inner());
        if !queued.insert(key) {
            return false;
        }
        self.sender.send(text.to_string()).is_ok()
    }
}

/// Prepare queued candidates one at a time until cancelled.
async fn run(
    mut receiver: mpsc::UnboundedReceiver<String>,
    mut cancelled: watch::Receiver<bool>,
    config: TtsConfig,
    preparer: &dyn Preparer,
) {
    let mut warned = false;
    loop {
        let text = tokio::select! {
            biased;
            _ = cancelled.wait_for(|cancelled| *cancelled) => return,
            next = receiver.recv() => match next {
                Some(text) => text,
                None => return,
            },
        };
        let preparation = preparer.prepare(text, config.clone());
        tokio::select! {
            biased;
            // Dropping the preparation is its cancellation.
            _ = cancelled.wait_for(|cancelled| *cancelled) => return,
            result = preparation => {
                if result.is_err() && !warned {
                    warned = true;
                    // Never the phrase, a secret, or the provider's response.
                    warn!("TTS pre-warm could not prepare a phrase; it is synthesized when it plays");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
