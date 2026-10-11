//! Lock-free, coherent operation/time pairs. Each lane has one writer; lanes
//! are sampled independently and do not claim a globally simultaneous view.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub(crate) enum Operation {
    PipeWait = 1, BytesArrived, RecordProcessing, JsonDecode, SignalObservation,
    ProviderParse, VerdictPublication, AnswerPublication, SummaryConstruction,
    RenderComputation, OutputSubmission, SemanticLogging, HookCallback,
    LifecycleCallback, Eof, JoinStart, JoinEnd, Cutoff, Settlement,
    StoragePublication, Idle, TerminalDelivery, DeliveryComplete, DeliveryFailed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct OperationSnapshot {
    pub(crate) operation: Operation,
    pub(crate) at_us: u64,
    pub(crate) started_before_run: bool,
}

pub(crate) struct OperationLane {
    origin: Instant,
    pair: AtomicU64,
}

impl OperationLane {
    pub(crate) fn new(origin: Instant) -> Self {
        Self { origin, pair: AtomicU64::new(0) }
    }

    pub(crate) fn record(&self, operation: Operation) {
        let micros = self.origin.elapsed().as_micros().min((u64::MAX >> 8) as u128) as u64;
        self.pair.store((micros << 8) | operation as u64, Ordering::Release);
    }

    pub(crate) fn snapshot_since(&self, origin: Instant) -> Option<OperationSnapshot> {
        let mut snapshot = self.snapshot()?;
        let at = self.origin + std::time::Duration::from_micros(snapshot.at_us);
        snapshot.started_before_run = at < origin;
        snapshot.at_us = at.saturating_duration_since(origin).as_micros().min(u64::MAX as u128) as u64;
        Some(snapshot)
    }

    pub(crate) fn snapshot(&self) -> Option<OperationSnapshot> {
        let pair = self.pair.load(Ordering::Acquire);
        let operation = match pair as u8 {
            1 => Operation::PipeWait, 2 => Operation::BytesArrived,
            3 => Operation::RecordProcessing, 4 => Operation::JsonDecode,
            5 => Operation::SignalObservation, 6 => Operation::ProviderParse,
            7 => Operation::VerdictPublication, 8 => Operation::AnswerPublication,
            9 => Operation::SummaryConstruction, 10 => Operation::RenderComputation,
            11 => Operation::OutputSubmission, 12 => Operation::SemanticLogging,
            13 => Operation::HookCallback, 14 => Operation::LifecycleCallback,
            15 => Operation::Eof, 16 => Operation::JoinStart, 17 => Operation::JoinEnd,
            18 => Operation::Cutoff, 19 => Operation::Settlement,
            20 => Operation::StoragePublication, 21 => Operation::Idle,
            22 => Operation::TerminalDelivery, 23 => Operation::DeliveryComplete,
            24 => Operation::DeliveryFailed, _ => return None,
        };
        Some(OperationSnapshot { operation, at_us: pair >> 8, started_before_run: false })
    }
}

pub(crate) fn increment(counter: &AtomicU64, amount: u64) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some(value.saturating_add(amount))
    });
}
