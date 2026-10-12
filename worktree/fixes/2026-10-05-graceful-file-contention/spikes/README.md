# Phase 1 spikes (S1-S3)

Throwaway probes behind the Phase 1 rulings R4-R6. The findings and rulings
are recorded in `../implementation-log.md` (Phase 1). These files are kept
only so a reviewer can rerun the probes. Nothing here is compiled by the
workspace.

| File | What it is |
| --- | --- |
| `probe.rs.txt` | `gfc-probe` source. Build it as `src/main.rs` of a scratch crate whose `Cargo.toml` is `probe.Cargo.toml.txt` (it declares its own `[workspace]`). `gfc-probe s1 <base> [<other-volume>]`, `gfc-probe s2 <base>`, `gfc-probe b1 <base> [<files>]` |
| `s3.sh` | Git move and final-unregister probe (`bash s3.sh`; creates and removes its own temp dir) |
| `results/` | Raw output, one file per host and run (`build-win` is the WSL2 guest, `windows` is the native Windows host). `run2-*` and `s2-*` are the authoritative S1/S2 runs. Files named `INVALID-run1-*` are kept for the record only: an orphaned `ping.exe` held its cwd in the source and skewed every later Windows S1 case |
