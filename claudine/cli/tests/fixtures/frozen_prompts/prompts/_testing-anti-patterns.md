## Testing Anti-Patterns

Each pattern below has broken a test or slowed one past the 5 s budget in this repository. An L1
test over 5 s counts as failing. CI runners have 2–4 cores and run a whole package at once, so a
test that takes 2 s alone can take 8 s there. Check new and changed tests against this list.

### Patterns that make tests slow

- **Capturing repository state from the process directory.** Nextest runs a test from inside this
  monorepo. Anything that discovers Git or package state from the current directory walks the whole
  checkout, at 0.4–6 s per call. The common sources are `ComposeContext::capture_for_dir(&current_dir())`,
  darkmatter's `test_request(…)`, and a test helper whose request directory falls back to the cwd.
  *Instead:* anchor the request or capture in the test's own temp dir. Make the few tests that need
  the real checkout ask for it explicitly.
- **Rebuilding an expensive context inside a loop.** A resolution context, a `RepositoryContexts`,
  or a `DarkmatterSchemas` built once per matrix cell repeats repository discovery for every cell.
  *Instead:* build each one once, outside the loop, when it doesn't depend on the loop variable.
- **One test that loops over a matrix of process spawns.** Each `claudine`/`md` spawn costs
  50–300 ms, and more on Windows. A single test spawning 150–300 of them takes 15 s or more.
  *Instead:* generate one `#[test]` per row with `macro_rules!`, so nextest spreads the rows across
  cores.
- **Spreading work over threads inside a test.** `std::thread::scope` fan-out inside a test
  multiplies against nextest's own parallelism and oversubscribes a 2-core runner. That turns a slow
  test into a timeout. *Instead:* split the work into separate tests.
- **Waiting out a real production deadline.** A drain budget, a cancellation deadline, or an
  acceptance window used at its production value (2–10 s) makes the test last that long.
  *Instead:* pass a short deadline through the internal entry point, or a `test-fixtures`-only seam.
  Then assert the evidence that the deadline branch fired, usually its error text, so a starved
  runner fails loudly rather than passing through another branch.
- **Leaving a background task without a responder.** A fixture with no fake peer can leave a
  background check holding a lock until its own timeout expires, and the next call blocks behind it.
  *Instead:* start a fake that answers every request the code under test can make, and wait on the
  real condition rather than setting state by hand.
- **Leaving a terminal query unanswered.** Windows ConPTY holds a child about 3 s waiting for an
  answer to its Device Attributes query. *Instead:* have the pseudoconsole host reply (see the `os`
  skill).
- **Copying a large fixture for every case.** Copying 25 files, including a 614 KB artifact, per
  matrix cell costs 50–200 ms each time. *Instead:* build one fixture per test and reset only what
  each case changes.
- **Running the full pipeline to test one narrow behavior.** Checking ANSI output against all 10
  providers does ten times the work of checking it against one. *Instead:* use the smallest
  fixture and command scope that still exercises the behavior.
- **Spawning a process for setup that could be a file write.** Seven `git config` calls, or a
  `.cmd` shim that adds `cmd.exe` to every call on Windows, add up quickly. *Instead:* write
  `.git/config` directly, and use an `.exe` stub on Windows.
- **Running the same CLI pass again just to read its result.** A test that runs `check` only to
  confirm state can usually call the library function in-process. *Instead:* run the binary once
  for the behavior under test, and observe the rest in-process.
- **Fully decoding every file in a repository scan.** Splitting thousands of files into lines to
  find a handful of needles wastes nearly all the work. *Instead:* skip any file whose text
  contains no needle before the per-line pass.
- **Ignoring what a slow test says about production.** The `pr` flow composed the same document
  three times, and every user paid for it. *Instead:* when a test is slow, look for redundant work
  in the code under test before tuning the test.

### Patterns that break tests (often on one OS only)

- **Building expected paths with raw `std::fs::canonicalize`.** On Windows it returns `\\?\C:\…`,
  while production returns the simplified `C:\…`. *Instead:* use
  `biscuit_file::canonicalize_simplified`.
- **Using rootless "absolute" fixture paths.** On Windows, `/repo/prompt.md` resolves against the
  current drive and renders as `file:///B:/repo/prompt.md`. *Instead:* use a drive-qualified path
  on Windows, or derive the path from a temp dir.
- **Running Windows tests from Git Bash.** Under MSYS, a `git` started by a test can crash with
  0xC0000005 and empty stderr. *Instead:* run cargo and nextest from PowerShell.
- **Assuming an asynchronous release has finished.** On Windows, a named pipe stays reachable for
  a moment after its server is dropped, so a client can connect to a dying instance. *Instead:*
  make shutdown wait for the resource to disappear, or synchronize on the condition you assert.
- **Defining a constant outside the `#[cfg]` block that uses it.** On the other OS it is dead code
  and fails `-D warnings`. *Instead:* define it inside the same `#[cfg(...)]` scope.
- **Adding an ambient-state read to production code without its guard entry.** A new
  `std::env::var` fails the context-construction guard tests. *Instead:* list it with a reason, or
  route it through the request snapshot.
- **Shortening a deadline until a loaded runner misses it.** *Instead:* keep a wide margin over the
  fake's poll interval, and assert which branch fired.
- **Sharing a mutable fixture across tests that run in parallel.** A route that edits fixture files
  can disturb another test's reads. *Instead:* give each test its own fixture, or run the mutating
  cases last on a restored copy within one test.

### Patterns that silently lose coverage

- **Splitting a loop into named tests without a guard.** A row added to the source table later gets
  no test. *Instead:* add a guard test that compares the macro's list with the table.
- **Moving a repository-file literal into a helper.** CI's test-input index maps an `include_str!`
  or path literal inside a `#[test]` to that test, but in a helper it widens the cell to the whole
  test binary. *Instead:* keep the literal inside each `#[test]` function.
- **Treating these as fixes for a slow test.** A `slow_` prefix, a nextest `slow-timeout` override,
  `#[ignore]`, `#[cfg]` that skips the slow OS, retries, or dropped cases all leave the test slow or
  unrun. *Instead:* make the test fast, or report why it can't be.
