# Agent Errors Seed Baselines

These files are the immutable, post-graduation copies of the Phase-A runtime
vocabularies. Each YAML document is a direct `ErrorVocabulary` value. The
deterministic research gate compares every historical row against this baseline
so research cannot silently remove, re-kind, or reorder seeded behavior after
the provider facts keys have been deleted.

Update a baseline only through a dedicated behavior-changing fix with explicit
adjudication. Ordinary research refreshes must not rewrite these files.
