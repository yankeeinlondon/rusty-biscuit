# REJECTED CLI run — retained as a negative control, NOT evidence

Captured at host load average **46 -> 69** (two concurrent agent build/test jobs).
The predeclared drift bracket rejected it; no number here may be quoted.

Why it is rejected (the bracket doing its job):

- Per-run dispersion exceeds the effect: `large_fm_detailed` measured
  41.47 +/- 26.71 ms — a standard deviation ~64% of the mean.
- Bracket drift (candidate_A vs candidate_B, identical binary) is **9.5% /
  17.7% / 20.0%** across the three cases, i.e. as large as or larger than any
  delta being claimed. Per the benchmark README, "A delta smaller than the
  measured bracket drift is not a result."
- The `large_fm_simple` control reported a +76.7% "regression" on a path whose
  `--diff` call graph is unchanged between the two binaries — mechanically
  impossible, and a direct demonstration that the run measured host load rather
  than code.

Retained because the evidence contract asks for raw vectors, including for runs
that fail their own gate: this is the reproducible proof that the quiet-host run
beside it was necessary. See `../summary.md` for the accepted measurement.
