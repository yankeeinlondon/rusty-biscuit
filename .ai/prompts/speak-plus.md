# Biscuit Speaks upgrade

Currently the `biscuit-speaks` package exports two utility functions:

- `speak_when_able`
- `speak`

Both have identical signatures but handle error conditions differently.

This functionality is wrapped around the `tts` crate.

## Refactoring `biscuit-speaks`

The original intent was to have a binary which would -- out of the box -- interrogate and use the host's TTS capabilities without any need for other external libraries. However, it's clear that `tts` _does_ need additional work when used on at least Linux to make it correctly.

This refactor will address this issue by:

- moving away from the `tts` crate completely
- Detect client TTS programs:
    - We will use the `InstalledTtsClients` struct from the Sniff library (in this monorepo) to quickly and confidently identify what TTS programs are available on the host.

---

