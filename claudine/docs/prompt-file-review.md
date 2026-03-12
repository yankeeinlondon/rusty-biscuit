# Prompt File Implementation Review

Overall, the `--prompt-file` switch feature is well-implemented and maps closely to the design document and plan. The internal structure (e.g., path resolution, Darkmatter composition, env extraction) and provider-specific prompt-delivery mechanism (`WrapperProfile::apply_prompt_body`) are clean and functionally correct.

However, there are a few missing components compared to the plan, as well as some opportunities to improve the code's ergonomics and performance.

## Missing Functionality & Test Coverage

1. **Missing Integration Tests**
   Phase 8 of the implementation plan explicitly calls for end-to-end integration tests to be added to `claudine/cli/tests/wrap_commands.rs` (such as testing Codex/Gemini/Goose/Claude args/stdin generation, env var propagation, conflict errors, etc.). Currently, there are no `--prompt-file` tests in `wrap_commands.rs`. The unit tests in `prompt_file.rs` are excellent, but they do not cover the orchestration layer.

2. **Missing Stdin Conflict Check**
   In `claudine/cli/src/commands/wrap/prompt_file.rs`, the function `detect_existing_prompt_source` is supposed to detect if the prompt source conflicts with provider-native prompts. The plan explicitly stated: *"For stdin-based providers, check if stdin is not a terminal (indicating piped input)."*
   Currently, the function completely omits the stdin check for `Provider::Claude` and `Provider::KimiCode`. It should ideally check `!std::io::stdin().is_terminal()` and return an error if a prompt file is specified while stdin is already piped.

## Ergonomics, Idiomatics, and Performance

1. **Single-Pass String Normalization**
   In `prompt_file.rs`, the `normalize_env_name` function allocates two strings (`replaced` and `collapsed`) and iterates over the characters twice:
   ```rust
   let upper = key.to_ascii_uppercase();
   let replaced: String = upper
       .chars()
       .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
       .collect();
   
   let mut collapsed = String::with_capacity(replaced.len());
   // ...
   ```
   This can be collapsed into a single pass and a single allocation:
   ```rust
   let mut collapsed = String::with_capacity(key.len());
   let mut prev_underscore = false;
   for c in key.chars() {
       let mut c = c.to_ascii_uppercase();
       if !c.is_ascii_alphanumeric() {
           c = '_';
       }
       if c == '_' {
           if !prev_underscore {
               collapsed.push(c);
           }
           prev_underscore = true;
       } else {
           collapsed.push(c);
           prev_underscore = false;
       }
   }
   ```

2. **Directory Traversal**
   The fallback mechanism for bare filenames (`search_repo_for_filename`) implements a manual recursive directory search `walk_dir_recursive`. While it successfully filters out standard hidden directories, `node_modules`, and `target`, it doesn't respect `.gitignore`. The design document correctly noted that the `ignore` crate would be a better choice for repo-wide searches. Adding `ignore = "0.4"` to `claudine/cli/Cargo.toml` and using `ignore::Walk` would prevent traversing deeply nested ignored directories (e.g., Python `venv` or Rust `target` dirs named differently).

3. **Vector Shifting (Goose Profile)**
   In `profile.rs`, the `apply_prompt_body` method for `Goose` calls `.insert()` twice at the same position:
   ```rust
   args.insert(pos + 1, prompt.to_string());
   args.insert(pos + 1, "-t".to_string());
   ```
   This shifts the tail of the vector twice, which is an `O(N)` operation each time. It's technically more idiomatic and performant to use `splice` to insert multiple elements at once, although in this context `args` is tiny so the performance impact is negligible.

4. **Redundant Path Check**
   In `prompt_file.rs` `resolve_prompt_file`:
   ```rust
   if !path.exists() { ... }
   if !path.is_file() { ... }
   ```
   `path.is_file()` inherently checks for existence. The `exists()` check is redundant unless strictly kept to differentiate the error message ("not found" vs "is not a regular file").

## Unrelated Test Fix Applied
During the review, we also noticed and fixed a failing test in the Claudine library (`events::environment::tests::from_empty_sniff_result`). The `impl From<SniffResult> for EnvironmentContext` was implicitly leaking the host environment by reading `PACKAGE_AREA` inside `From`. We refactored `apply_wrapper_package_context` out of the `From` implementation and moved it directly into `detect_environment` to ensure the library's unit tests stay isolated.
