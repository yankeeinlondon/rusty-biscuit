# Claudine CLI Argument Parsing Re-Review

After reviewing the recently implemented changes to the CLI argument parsing and error reporting (`main.rs`, `wrap/mod.rs`, `output.rs`), the architecture is significantly improved. The two-pass parsing with `ignore_errors(true)` successfully allows native proxying without the mandatory `--` separator, and the structured error reporting cleanly wraps agent failures. 

However, there are a few remaining edge cases and minor formatting issues to address.

## 1. Duplicated "Error:" Prefix in `try_format_cli_error`

In `claudine/cli/src/output.rs`, the `try_format_cli_error` function identifies lines with prefixes like `error: `, `Error: `, or `fatal: `. However, when returning the formatted string, it prepends the styled `Error:` label to the *original* line.

For example, `error: unrecognized argument '--foo'` will be rendered as:
`<red><bold>Error:</bold></red> error: unrecognized argument '--foo'` (printing "Error: error: ...").

**Recommendation:** 
Use `strip_prefix` to remove the recognized string (ignoring case or handling specific known prefixes) before prepending the styled `<red><bold>Error:</bold></red>` label, ensuring a cleaner output without double labels.

## 2. POSIX `--` Escape Hatch Violation

The documentation added to `WrapperArgs` notes that Claudine flags are extracted from the passthrough bucket so they "work whether placed before or after `--`."

While this acts as a helpful fallback because `trailing_var_arg = true` eats everything after positional arguments, scanning *everything* in `passthrough` breaks the standard POSIX convention where `--` explicitly means "stop parsing options and treat the rest as literal arguments for the wrapped program."

If an underlying agent introduces a flag that collides with a Claudine flag (e.g., if Goose adds a `--silent` or `--repo` flag), the user currently has **no way** to send that flag to the agent. Claudine's manual extractor will continuously "steal" it from the passthrough bucket, even if the user explicitly placed it after `--`.

**Recommendation:** 
Modify `extract_wrapper_flags_from_passthrough` so it respects the `--` boundary. If it encounters a standalone `--` string in the passthrough vector, it should immediately stop scanning for Claudine flags and leave the remainder of the vector untouched for the agent. (Note: you may need to pass the raw `std::env::args()` or check for `--` before `clap` strips it to properly establish this boundary).

## 3. Dangling Value-Taking Flags Silently Forwarded

Because `ignore_errors(true)` suppresses `clap`'s strict validation, if a user types a command like:
`claudine gemini --operation` (without providing the required value)

`clap` will simply ignore the parsing failure and dump `--operation` into the `passthrough` vector. The manual `extract_wrapper_flags_from_passthrough` loop will see `--operation`, fail to find `args.get(i + 1)`, and therefore leave `--operation` in the passthrough vector. This dangling flag gets silently forwarded to the agent, which will likely crash with a confusing error.

**Recommendation:** 
In `extract_wrapper_flags_from_passthrough`, if a value-taking flag like `--operation` or `--op` is encountered at the very end of the arguments list (i.e., without a following value), Claudine should explicitly abort with a clear error ("missing value for --operation") error, rather than silently leaking the malformed wrapper flag to the underlying agent.
