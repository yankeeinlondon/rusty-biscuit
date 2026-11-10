# Dynamic slot fixtures (Darkmatter)

`claudine/` mirrors the shipped sandboxing layout: `providers/sandboxing.yaml`
loads `claudine/docs/os.yaml` (`macOS, Linux, Windows`) three folders up, and
`providers/provider.md` is a document that validates once the slot resolves.

`tests/dynamic_slots_fixtures/mod.rs` copies this tree into a fresh Git
repository and produces the tracked, dirty, untracked, and ignored variants the
approval matrix needs. Each matrix cell changes one thing in that control
repository.
