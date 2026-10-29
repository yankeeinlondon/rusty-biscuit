//! A controllable [`Preparer`] for pre-warm tests.
//!
//! Nothing here touches a speech provider, the audio cache, or the network.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use biscuit_speaks::TtsConfig;
use tokio::sync::{Semaphore, watch};

use super::{PrepareFuture, Preparer, PrewarmError};

/// One observed [`Preparer::prepare`] call.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PrepareCall {
    pub text: String,
    pub config: TtsConfig,
}

/// A recording, optionally blocking, optionally failing [`Preparer`].
///
/// A call is recorded when `prepare` is invoked, before its future is polled.
/// It is counted as completed only when its future runs to the end, so a
/// preparation dropped while blocked is observable as started but never
/// completed.
#[derive(Clone)]
pub(crate) struct FakePreparer {
    state: Arc<FakeState>,
}

struct FakeState {
    calls: Mutex<Vec<PrepareCall>>,
    completed: Mutex<Vec<String>>,
    failing: Mutex<HashSet<String>>,
    /// `None` completes every call at once; `Some` holds each call until
    /// [`FakePreparer::release`] grants it a permit.
    gate: Option<Semaphore>,
    call_count: watch::Sender<usize>,
    completed_count: watch::Sender<usize>,
}

impl FakePreparer {
    /// Every call completes as soon as it is polled.
    pub(crate) fn recording() -> Self {
        Self::with_gate(None)
    }

    /// Every call waits for [`Self::release`] before it completes.
    pub(crate) fn blocking() -> Self {
        Self::with_gate(Some(Semaphore::new(0)))
    }

    fn with_gate(gate: Option<Semaphore>) -> Self {
        Self {
            state: Arc::new(FakeState {
                calls: Mutex::new(Vec::new()),
                completed: Mutex::new(Vec::new()),
                failing: Mutex::new(HashSet::new()),
                gate,
                call_count: watch::Sender::new(0),
                completed_count: watch::Sender::new(0),
            }),
        }
    }

    /// Make every call for `text` fail with [`PrewarmError::Prepare`].
    pub(crate) fn failing_on(self, text: impl Into<String>) -> Self {
        self.state.failing.lock().unwrap().insert(text.into());
        self
    }

    /// Let `count` blocked calls complete, in the order they reach the gate.
    pub(crate) fn release(&self, count: usize) {
        if let Some(gate) = &self.state.gate {
            gate.add_permits(count);
        }
    }

    pub(crate) fn calls(&self) -> Vec<PrepareCall> {
        self.state.calls.lock().unwrap().clone()
    }

    pub(crate) fn texts(&self) -> Vec<String> {
        self.calls().into_iter().map(|call| call.text).collect()
    }

    /// Texts whose preparation ran to the end, successfully or not.
    pub(crate) fn completed(&self) -> Vec<String> {
        self.state.completed.lock().unwrap().clone()
    }

    /// Wait until at least `count` calls have been made.
    pub(crate) async fn wait_for_calls(&self, count: usize) {
        let mut receiver = self.state.call_count.subscribe();
        receiver
            .wait_for(|calls| *calls >= count)
            .await
            .expect("the fake owns the sender");
    }

    /// Wait until at least `count` calls have run to the end.
    ///
    /// A call is made before its future is polled, so a test that drops the
    /// worker after [`Self::wait_for_calls`] can cancel the last call before
    /// it completes; wait here before asserting on [`Self::completed`].
    pub(crate) async fn wait_for_completed(&self, count: usize) {
        let mut receiver = self.state.completed_count.subscribe();
        receiver
            .wait_for(|completed| *completed >= count)
            .await
            .expect("the fake owns the sender");
    }
}

impl Preparer for FakePreparer {
    fn prepare(&self, text: String, config: TtsConfig) -> PrepareFuture {
        self.state.calls.lock().unwrap().push(PrepareCall {
            text: text.clone(),
            config,
        });
        self.state.call_count.send_modify(|calls| *calls += 1);
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            if let Some(gate) = &state.gate {
                gate.acquire().await.expect("the gate is never closed").forget();
            }
            let failed = state.failing.lock().unwrap().contains(&text);
            state.completed.lock().unwrap().push(text);
            state.completed_count.send_modify(|completed| *completed += 1);
            if failed { Err(PrewarmError::Prepare) } else { Ok(()) }
        })
    }
}
