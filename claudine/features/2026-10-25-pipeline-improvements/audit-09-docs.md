---
kind: implementation-log
---

# Documentation audit: 2026-10-19-speak-prepare

Updated **1 existing canonical document**; created **0 documents**.

`biscuit-speaks/docs/tts-caching.md` is the canonical entry point for every
requirement in this specification. The completed specification now lists that
implicit repository-relative path under `canonical_docs`, and the page declares
`kind: canonical-documentation`. The specification body is unchanged.

| Functionality | Canonical coverage |
| --- | --- |
| Uniform preparation API and provider selection | Preparing Speech; What gets prepared. Added the direct `TtsExecutor::prepare` contract, `Send` future, no-op default, feature availability, unsupported-provider behavior, and normal selection rules. |
| Returned builder versus fresh builder | The returned object and a fresh object. Existing diagram and state/setter rules retained; added a complete explicitly selected Kokoro example and pinned the existing macOS example to Say. |
| Cache identity, default resolution, and speed/volume | Audio File Cache; Provider Integration; Say's system defaults. Existing provider-specific rules retained, including the unresolved normal-speed Say rate. |
| Retained-first playback, fallback, and helper handoff | Playback after preparation. Existing ordinary/detached playback rules retained; clarified that queued playback can wait behind earlier audio even after a cache hit. |
| ElevenLabs default selection records | The ElevenLabs default-voice record. Existing account-label requirement, non-secret identity, atomic publication, 24-hour trust window, refresh, and miss behavior verified against current code. |
| Dry run, cancellation, concurrency, and absence of playback | What gets prepared; Atomic Writes. Existing dry-run spellings retained; clarified awaited preparation versus caller-owned background execution and cleanup after failed cache publication. |

The old preparation introduction promised the phrase would play immediately.
The replacement states what preparation guarantees: synthesis may be skipped,
while Playa's ordered queue still controls playback time. An illustrative cache
filename also used non-hexadecimal characters despite claiming a hexadecimal
key; it now uses a valid 16-character hexadecimal example.

The concurrent test audit found and repaired a temporary-file leak on failed
cache publication. The Atomic Writes paragraph now describes the cleanup
contract without depending on the temporary-filename strategy, and explicitly
covers failed writes, flushes, and publication while preserving existing entries.

Validation inspected `Speak`, `TtsExecutor`, provider resolution paths,
`audio_cache`, Say's system-default resolver, and ElevenLabs' snapshot reader.
The examples use exported types and actual `TtsConfig` builder methods; no
speech, network request, or synthesis process was run for documentation checks.
Reference, frontmatter kind, section-anchor, unchanged-spec-body checks, and
`git diff --check` passed. No feature/fix references were added to the canonical
page, and no additional canonical page or skill edit was needed.
