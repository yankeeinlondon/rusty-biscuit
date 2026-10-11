# Verification Fidelity

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

A **tier** (L1, L2, L3, browser, real, slow) says what resource a test needs.
A **rung** says what a passing test proves. Choose the tier from the rung you
need, never the other way around.

| Rung | Name       | Who authored what                                         |
|------|------------|-----------------------------------------------------------|
| F1   | Synthetic  | We wrote the input and the expected output.               |
| F2   | Witnessed  | A real consumer observes our output; our stimulus drives. |
| F3   | Native     | The real counterparty originates the input.               |

## Choosing a rung when you write a test

1. Name the **boundary** the requirement depends on: the foreign thing whose
   behavior the code assumes (terminal key encoder, provider wire format,
   browser layout, OS table, another tool's file format).
2. Ask whether an F1 test could pass while the real boundary behaves
   differently. If yes, F1 alone is not evidence for that requirement.
3. Pick the lowest rung that falsifies the assumption, then the tier that
   reaches it (`level2_`/`browser_` for F2, `level3_`/`real_` for F3, or a plain
   L1 test that compiles and runs generated code against a loopback server).
4. Keep the F1 tests too. They are fast and pin the logic; F2 and F3 pin the
   assumption.

## Traps

- **The tier name is not the rung.** A `real_` test that only reads a local
  fixture is F1. A `level2_` test that only checks launch is barely F2. Judge
  by what the assertions depend on.
- **A recorded response is F3 only for the format it captured.** Store when and
  from where it was recorded, and refresh it when the provider changes.
- **A stub you wrote cannot disprove your reading of the API.** Wiremock
  responses are F1 for the wire format, however many tests share them.
- **An unreachable boundary is acceptable only when stated.** For a paid API or
  hardware CI lacks, the review records the assumption and what it rests on.

The boundary catalog (each recurring boundary and the rung it needs) lives in
`prompts/_test-rigor.md`, and the explanation for readers new to the repository
is in `docs/testing-strategy.md` under "Levels say what a test needs; fidelity
says what it proves."
