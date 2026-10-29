# Frozen A3 document

`implement-plan.md` is a byte-for-byte copy of the repository's
`prompts/_implement/implement-plan.md`, copied on 2026-10-26 from the tree at
commit `c582c8bc0`. It is the largest lifecycle document in the
repository (143 frontmatter lines: `initialize`, `start`, `success`,
`blocked`, `failure`, and a `loop` with a `gate`), and the realistic
anchor A3 of the `pipeline_work` benches (lib) and the `pipeline_work_a3`
bench (DMLS) is built from it.

It is frozen. Never refresh it to follow the prompt: the benches compare runs
over time, and a moving document would change what is measured.
