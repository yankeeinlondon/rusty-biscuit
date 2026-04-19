# Review 2: OpenCode Model Resolution & AgentError Consistency

The implementation has been significantly improved. The precedence logic is now cleanly encapsulated via `source.apply_to_args`, and the requested markup and footer text have been added.

However, there are still a few functional gaps and rendering bugs that need to be addressed.

## 1. Bug: Double-Rendering ANSI into Prose
In `claudine/cli/src/output/error_report.rs`, the `render()` method currently calls `.render(term)` on the `Status` (Did you mean:) and `UnorderedList` components, pushing the resulting ANSI-escaped strings into `parts`. These parts are then joined and passed into a `Prose` component.
`Prose` performs text wrapping and layout calculations but does *not* understand pre-rendered ANSI escape sequences. Passing them into `Prose` treats the invisible escape characters as visible, which corrupts line-wrapping and layout.
**Recommendation:** Use the `Compose` component from `biscuit_terminal::components::compose::Compose` to assemble the error block. `Compose` can hold multiple `RenderableContent` parts (including your `Prose` text, `Status`, and `UnorderedList`) and render them correctly, allowing the `BlockQuote` to wrap the composite layout.

## 2. Bug: Double Markup in Suggestions
In `AgentErrorReport::no_model_provided`, the strings provided to `suggestions` contain `<yellow>` tags (e.g., `"set <yellow>OPENCODE_MODEL</yellow>..."`). However, the `render()` method maps over `suggestions` and wraps each item in *another* `<yellow>` tag (`format!("<yellow>{s}</yellow>")`). This results in invalid nested tags (`<yellow>set <yellow>...`).

## 3. Spec Divergence: Order of "No Model Provided"
The spec defined the layout as:
1. Body text
2. List of overrides
3. (Blank line)
4. Footer text

Currently, the footer text is passed in the `detail` field. `render()` places `detail` *before* the `suggestions` list, causing the footer to appear in the wrong place.

## 4. Spec Divergence: "Did you mean:" Prefix
The override methods for "No Model Provided" are being passed via the `suggestions` field. `render()` assumes `suggestions` are for "Invalid Model Specified" and unconditionally prefixes them with a `Did you mean:` warning. This header was not requested for the pre-flight error and does not make semantic sense in that context.
**Recommendation:** You might consider separating the concept of generic "suggestions" (which gets the "Did you mean:" header) from a generic bulleted list, or allow the header to be overridden/omitted.

## 5. Classification Fallback Logic
In `classify_native_cli_error`, if `stderr` indicates a model not found error (e.g., contains "providermodelnotfounderror") but both `suggestions` and `location` evaluate to `None`, the function falls through and returns `None`. This causes Claudine to fall back to the generic `{provider} exited with error code {exit_code}` message. It should still return an `invalid_model` block with a generic fallback location (e.g., `"the command line"`) in this edge case.