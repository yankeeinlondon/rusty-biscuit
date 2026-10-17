## Test Rigor — Verification Fidelity (F1 / F2 / F3)

Test count is not test rigor. Phrases like "covered by substantial unit and integration tests" are
banned from this review unless you can pair each user-facing requirement with a **boundary** and the
**fidelity rung** at which that boundary was crossed.

A **boundary** is a foreign thing whose behavior we assume: a terminal's key encoder, a provider's
wire format, a browser's layout engine, the OS routing table, an audio device, another tool's file
format. A test is only as rigorous as the independence of that foreign thing from our own
assumptions. Ask of every test: *who authored each side of the boundary?*

- **F1 — Synthetic.** We wrote the input and we wrote the expected output. Unit tests, snapshots of
  our own output, a PTY fed bytes we manufactured, a wiremock stub we scripted, a canned command
  transcript. Necessary and fast, but it can only confirm that our code agrees with our beliefs. It
  cannot catch "WezTerm does not emit bare-modifier press events because we forgot to push
  `REPORT_ALL_KEYS_AS_ESCAPE_CODES`", or "the provider paginates with a header, not the body field
  we stubbed."

- **F2 — Witnessed.** A real consumer or medium observes what we produce, but our stimulus still
  drives it. A real terminal renders our output and we read the pane back; headless Chrome computes
  the style of our HTML; generated client code is compiled by `rustc` and exercised against a real
  local server. This proves our output survives a real consumer. It does not prove the real world
  would have sent us that input.

- **F3 — Native.** The real counterparty originates the input. OS key events pass through the
  terminal's own encoder; a live provider answers a real request; the real host supplies its routing
  table or device list. This is the only rung that can falsify an assumption on the *input* side of
  the boundary. A **recorded capture** of a real counterparty (a cassette, a saved response) counts
  as F3 for the wire format it captured, but record its date and source, and treat it as stale-able:
  it proves what the counterparty did then.

### Boundary catalog

Name the boundary, then check the rung it needs. Recurring boundaries in this monorepo:

| Boundary (the foreign thing) | Typical F1 | F2 | F3 |
| --- | --- | --- | --- |
| Terminal input encoding | PTY fed manufactured bytes | tmux/WezTerm/Kitty injecting bytes through their CLI | OS key events (`cliclick`, `xdotool`) |
| Terminal rendering (glyph width, SGR, scroll) | String/snapshot of emitted escapes | Pane text captured from a real emulator | — (F2 is the ceiling) |
| Browser rendering and layout | HTML string assertions | Computed styles in headless Chrome | Real user interaction in a real browser |
| HTTP/WebSocket API wire format | Stubbed responses we wrote; hand-written spec fixture | Generated client compiled and run against a local server implementing the spec | Live provider call (`real_`), or a dated recorded response |
| Generated code | Generator output compared to expected text | Output compiled and its behavior exercised | Compiled against a vendor's published schema or SDK sample |
| Host discovery (OS, network, devices) | Parsing a canned command transcript | Running the real subprocess against a fixture | Reading the real host table or device |
| LLM or agent CLI behavior | Fixture transcripts | The real CLI under a scripted session | A live provider call |
| Audio / hardware output | Buffer contents | Real backend, output discarded | Real device |
| File and protocol formats from other tools | Parsing samples we authored | Round trip through the other tool | Samples produced by the other tool |

If a boundary is missing from the table, name it in the review and place it on the rungs the same
way.

### How rungs relate to execution tiers

Tiers (`level2_`, `level3_`, `browser_`, `real_`, `slow_`, and the `just test-*` recipes) say what
resource a test needs in order to run. Rungs say what a passing test proves. They correlate, not
coincide:

- `level2_` and `browser_` tests usually reach F2; `level3_` and `real_` tests usually reach F3.
- An L1 test is usually F1, but an L1 test that compiles generated code and runs it against a
  loopback server is F2.
- A `real_` test that only reads a local fixture is F1. A `level2_` test that only checks the program
  launched is barely F2. Judge the rung by what the assertions depend on, never by the test's name.

> **Note:** always make sure your F2 and F3 tests open application windows _without_ focus and
> ideally hidden. Not doing this will result in flaky tests because your tests will be getting stray
> input from the host's users. This is made easy to do; just be sure you're using the provided test
> framework correctly.

### Reviewing

For each requirement that asserts user-observable behavior, name its boundary and classify the
strongest rung present. Call out any mismatch:

- "Spec requires modifier-press to surface badges" + only F1 tests = **gap, not "ready"**. The
  input-encoding boundary was never crossed for real; this needs F3.
- "Spec requires `^X` badges with specific colors" + F1 style assertions only = needs F2 capture
  from a real terminal.
- "Spec requires cursor pagination" + F1 stubs of a body field we invented = needs F3 evidence (live
  or dated capture) that the provider really paginates that way.
- "Spec requires hotkey chord activation" + F2 in tmux and an F1 chord-byte test = fine, because the
  requirement is about bytes arriving, not about a key being pressed.
- A requirement that never crosses its boundary for real because the boundary is out of our hands
  (a paid API, hardware CI lacks) is acceptable only when the review states the assumption and the
  evidence it rests on.

A feature MAY be marked production-ready only when each user-observable requirement of the spec has
at minimum the rung appropriate for its boundary. Reviewers MUST list any requirement whose strongest
test is below that rung under "Findings" with severity at least "high".

> **Note:** this monorepo provides a test framework at tools/test-toolkit and you are expected to use it
