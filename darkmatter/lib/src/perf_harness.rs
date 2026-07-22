//! Test-only measurement harness for the 2026-07-15 performance follow-up.
//!
//! Several checkpoints in that feature target **crate-private** functions. A
//! `benches/*` target is a separate compilation unit and can only see `pub`
//! items, so those checkpoints cannot be measured from Criterion without
//! widening the public API — which the feature's compatibility invariant 2
//! bars. Phase 10's answer was a *temporary* in-crate harness that was deleted
//! after capture; that is precisely what left findings 25, 35.3, 35.5, 35.6 and
//! 35.7 with unrecoverable observations and unreproducible claims (review-2,
//! "Several benchmark dispositions still lack retained raw samples").
//!
//! This module is the retained replacement. It is `#[cfg(test)]`, so it adds no
//! public API and ships in no release artifact, yet it lives in the crate and
//! can therefore call private functions directly.
//!
//! ## Interleaved sampling
//!
//! The measurement host is shared. Identical unmodified code has drifted +50%
//! across runs, and a cross-run comparison under load once manufactured a
//! systematic-looking −19% shift across every benchmark in a binary. So a
//! baseline and its candidate are never compared across processes here:
//! [`Harness::interleaved_pair`] alternates them **sample by sample inside one
//! process**, so both see the same thermal and scheduling conditions and the
//! ratio survives whatever the absolute numbers do. No drift bracket is needed
//! because there is no cross-run comparison to bracket.
//!
//! ## Output format
//!
//! Vectors are written as Criterion's `sample.json` shape
//! (`{sampling_mode, iters, times}`, parallel arrays, `times[i]` nanoseconds for
//! a batch of `iters[i]` iterations). That is the shape the feature's
//! `benchmarks/recompute.ts` already reads, so retained vectors from this
//! harness recompute through the same committed tool as the Criterion ones:
//!
//! ```text
//! bun recompute.ts raw/<checkpoint>/<run-id>
//! ```
//!
//! ## Notes
//!
//! Harness tests are `#[ignore]`d and additionally gated on `DM_PERF_RAW_DIR`,
//! so the ordinary `just test` gate neither runs nor is slowed by them. They are
//! invoked explicitly with `--run-ignored` and that variable set; each writes
//! `<name>-sample.json` into the run-record directory it names.

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

/// A per-observation vector in Criterion's retained `sample.json` shape.
#[derive(Debug)]
pub(crate) struct SampleVector {
    iters: Vec<u64>,
    times: Vec<f64>,
}

impl SampleVector {
    /// Median per-iteration nanoseconds — the same statistic `recompute.ts`
    /// derives from the retained vector, computed here only so a harness can
    /// print a progress line.
    pub(crate) fn median_nanos(&self) -> f64 {
        let mut per_iter: Vec<f64> = self
            .times
            .iter()
            .zip(&self.iters)
            .map(|(time, iters)| time / *iters as f64)
            .collect();
        per_iter.sort_by(|a, b| a.partial_cmp(b).expect("timings are finite"));
        let middle = per_iter.len() / 2;
        if per_iter.len().is_multiple_of(2) {
            (per_iter[middle - 1] + per_iter[middle]) / 2.0
        } else {
            per_iter[middle]
        }
    }

    /// Serializes to Criterion's `sample.json` shape.
    fn to_json(&self) -> String {
        let iters = self
            .iters
            .iter()
            .map(|i| format!("{i}.0"))
            .collect::<Vec<_>>()
            .join(",");
        let times = self
            .times
            .iter()
            .map(|t| format!("{t}"))
            .collect::<Vec<_>>()
            .join(",");
        format!("{{\"sampling_mode\":\"Flat\",\"iters\":[{iters}],\"times\":[{times}]}}")
    }
}

/// A retained measurement harness writing raw vectors into one run record.
pub(crate) struct Harness {
    dir: PathBuf,
    warmup: usize,
    samples: usize,
    batch: u64,
}

impl Harness {
    /// Builds a harness from `DM_PERF_RAW_DIR`, or `None` when unset.
    ///
    /// Returning `None` rather than panicking is what lets a harness test be a
    /// normal `#[ignore]`d test: the ordinary gate can compile and (if someone
    /// passes `--run-ignored`) run it as a no-op, while a measurement run opts
    /// in by naming its run-record directory.
    pub(crate) fn from_env(samples: usize, batch: u64) -> Option<Self> {
        let dir = PathBuf::from(std::env::var_os("DM_PERF_RAW_DIR")?);
        std::fs::create_dir_all(&dir).expect("run-record directory creatable");
        Some(Self {
            dir,
            warmup: 3,
            samples,
            batch,
        })
    }

    /// Times one batch of `self.batch` iterations, in nanoseconds.
    fn time_batch(&self, operation: &mut impl FnMut()) -> f64 {
        let start = Instant::now();
        for _ in 0..self.batch {
            operation();
            black_box(());
        }
        start.elapsed().as_nanos() as f64
    }

    /// Samples `baseline` and `candidate` **alternately within each sample
    /// round**, then writes `<baseline_name>-sample.json` and
    /// `<candidate_name>-sample.json`.
    ///
    /// Callers must assert baseline/candidate equivalence *before* calling: a
    /// ratio between two functions that disagree is not a result.
    pub(crate) fn interleaved_pair(
        &self,
        baseline_name: &str,
        mut baseline: impl FnMut(),
        candidate_name: &str,
        mut candidate: impl FnMut(),
    ) -> (SampleVector, SampleVector) {
        for _ in 0..self.warmup {
            self.time_batch(&mut baseline);
            self.time_batch(&mut candidate);
        }

        let mut baseline_times = Vec::with_capacity(self.samples);
        let mut candidate_times = Vec::with_capacity(self.samples);
        for _ in 0..self.samples {
            baseline_times.push(self.time_batch(&mut baseline));
            candidate_times.push(self.time_batch(&mut candidate));
        }

        let iters = vec![self.batch; self.samples];
        let baseline_vector = SampleVector {
            iters: iters.clone(),
            times: baseline_times,
        };
        let candidate_vector = SampleVector {
            iters,
            times: candidate_times,
        };
        self.write(baseline_name, &baseline_vector);
        self.write(candidate_name, &candidate_vector);
        (baseline_vector, candidate_vector)
    }

    /// Samples one operation and writes `<name>-sample.json`.
    ///
    /// For a cost model or a profile stage, where there is no baseline/candidate
    /// pair to interleave.
    ///
    /// Retained with no current caller: every harness so far is a pinned
    /// baseline/candidate pair. This is part of the harness API the module is
    /// kept for, so it is allowed rather than deleted — `clippy --all-targets --
    /// -D warnings` would otherwise fail the lint gate on `dead_code`.
    #[allow(dead_code)]
    pub(crate) fn single(&self, name: &str, mut operation: impl FnMut()) -> SampleVector {
        for _ in 0..self.warmup {
            self.time_batch(&mut operation);
        }
        let times = (0..self.samples)
            .map(|_| self.time_batch(&mut operation))
            .collect();
        let vector = SampleVector {
            iters: vec![self.batch; self.samples],
            times,
        };
        self.write(name, &vector);
        vector
    }

    fn write(&self, name: &str, vector: &SampleVector) {
        let path = self.dir.join(format!("{name}-sample.json"));
        std::fs::write(&path, vector.to_json())
            .unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
        println!(
            "  {name}: median {:.4} µs/iter -> {}",
            vector.median_nanos() / 1000.0,
            path.display()
        );
    }
}

/// Reads a committed manifest fixture by stem (Architecture Decision A: the
/// measured bytes are frozen and hashed in `benchmarks/manifest.yaml`).
pub(crate) fn fixture_text(stem: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../features/2026-07-15-performance-followup/benchmarks/fixtures")
        .join(format!("{stem}.md"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {} readable: {e}", path.display()))
}
