• The audit confirms there are substantial opportunities to make CI both faster and less flaky without increasing
timeouts.

Current watchdog status:

- Still running continuously.
- macOS validation is green.
- WSL validation is green.
- Native Windows is pending because its configured target disk has 22.2 GiB free versus the repository’s 50 GiB
  preflight requirement. The watchdog is handling the bounded verification path.

- The current rendezvous repair remains isolated to two files and has not been disturbed.

Highest-value improvements:

1. Adaptive CI batching
   A full-scope run currently expands to roughly 477 jobs, including 65 Linux archive builds and 65 WSL guests. For
   wide changes, packages should be divided into a few cost-balanced batches while retaining per-package results. WSL
   could use perhaps four shared archives/guests instead of 65. This is the largest likely reduction in runtime and
   transient-failure exposure.

2. Build once for Linux and WSL
   The WSL workflow says its archived binaries are identical to the Linux leg, yet it performs a separate archive
   build for every package. Reusing the Linux build would eliminate substantial redundant compilation.

3. Replace fixed sleeps with deterministic synchronization
   Several Claudine sequence tests use one-second sleeps and elapsed-time thresholds to prove concurrency. File
   barriers or explicit readiness signals can prove the same behavior in milliseconds without depending on runner
   speed. The obvious sequence-group cases alone consume over five seconds per Claudine run.

4. Narrow test serialization
   All 2,370 Claudine CLI L1 tests are currently serialized, taking approximately 551 seconds locally. Only
   subprocess-, signal-, or repository-discovery-heavy tests need restricted concurrency. Parser and pure unit tests
   can run normally. The same principle applies to Sniff’s broad Windows serialization and Claudine’s signal-
   sensitive L2 tests.

5. Remove fragile WSL streaming downloads
   Retrying curl ... | tar is unsafe after a partial archive has reached tar, matching a recent truncated-download
   failure. Prefer bundling pinned just and cargo-nextest binaries into the WSL archive. At minimum, download to a
   file, validate it, and extract atomically.

6. Reduce artifact count
   JUnit and status data are currently uploaded as separate artifacts for many cells. Combining them would remove
   hundreds of artifact operations and reduce the likelihood of artifact enumeration and quota failures.

7. Eliminate repeated repository scans
   Some source-guard and discovery tests independently walk the repository, with several taking roughly 1.6 seconds
   each. Combining related assertions into one scan preserves coverage while reducing execution time and contention.

8. Prevent known regressions
   Add a fast static guard prohibiting CLI tests from spawning cargo run. That pattern caused the original biscuit-
   speaks timeout through Cargo build-lock contention.

I recommend implementing deterministic test barriers and safe WSL provisioning first, then narrowing serialization,
followed by adaptive batching/shared builds. No timeout increases or retry-to-green behavior are recommended, and I
made no audit-related code changes while the watchdog’s repair is active.
