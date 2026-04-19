## Validation

I re-checked the three findings from `review.md` against the current implementation.

1. The false-positive `--` validator issue appears fixed. [`claudine/cli/src/commands/wrap/profile.rs:1985`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/profile.rs:1985) now skips the first argument after `--`, which matches OpenCode's intentional `["--", prompt]` delivery shape and avoids warning on bullet-list prompts.

2. The OpenCode model-resolution helper is now decoupled from mutable process state for unit testing. `apply_opencode_model_resolution(...)` takes an injected [`OpenCodeEnvSnapshot`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/profile.rs:62), and the previously failing unit tests now use explicit snapshots instead of mutating `HOME` / `OPENCODE_MODEL`. I validated this with:
   `cargo test -p claudine-cli wrap::profile -- --nocapture`
   Result: `79 passed; 0 failed`

3. The missing direct-wrap spawn coverage has been added. The new integration test file [`claudine/cli/tests/wrap_direct_argv.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/wrap_direct_argv.rs:1) now verifies:
   OpenCode direct-wrap argv puts `run`, `--model`, `--format`, and YOLO flags before `--`, with the bullet-list prompt after it
   Goose direct-wrap argv contains exactly one `run`

I validated that coverage with:
`cargo test -p claudine-cli --test wrap_direct_argv -- --nocapture`
Result: `2 passed; 0 failed`

## Additional Suggestions

No additional review findings at this point. The issues from `review.md` appear to have been implemented and validated.
