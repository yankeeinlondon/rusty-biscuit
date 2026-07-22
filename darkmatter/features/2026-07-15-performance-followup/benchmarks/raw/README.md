# Dated run records

Per-run measurement facts live here, one directory per measurement, kept
separate from the immutable fixture identity in `../manifest.yaml` (AD-A).

```
raw/<checkpoint>/<run-id>/
├── build.log        # build + harness log
├── <case>.json      # raw per-case samples (e.g. hyperfine JSON)
└── ...
```

Each run record documents baseline/candidate commits, exact commands, release
profile, host facts, environment, TTY mode, warm-up, sample count,
statistic/dispersion, predeclared thresholds (declared before the baseline is
captured), and retained raw result files. `../../results.md` links each
disposition to its run record. See `../README.md` for the full contract.

`<run-id>` is `run-<UTC timestamp>`. Interactive (PTY) and piped (redirected
CLI) measurements are recorded as separate cases.
