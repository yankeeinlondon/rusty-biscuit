//! Test 8: a persistent cache entry written under the previous extractor
//! fingerprint is not served after the upgrade, and the entry written by the
//! upgraded run is served on the next run. Works on the cache files directly,
//! with `TMPDIR` (`TMP`/`TEMP` on Windows) pointed at a test-owned directory.
