# Gotchas & troubleshooting

## 1) Compile errors / trait mismatches / regex types don’t align

**Symptom:** confusing compilation failures after enabling syntect features.

**Cause:** syntect and two-face compiled with different regex backends.

**Fix:** ensure both use the same mode.

* syntect oniguruma → two-face `syntect-default-onig`
* syntect fancy-regex → two-face `syntect-default-fancy`

See: [Setup & feature flags](setup-and-features.md)

## 2) Missing syntaxes when using fancy-regex

**Symptom:** `find_syntax_by_extension("…")` returns `None` for a language you expect.

**Cause:** some syntaxes rely on regex features not supported by `fancy-regex` and are excluded.

**Fix options:**

* Switch to oniguruma backend
* Provide fallback to plain text
* Document which syntaxes are supported in your environment

## 3) Wrong newline mode

* `extra_newlines()` is generally safest and matches common syntect usage.
* `extra_no_newlines()` can change parsing behavior; use only if you have a concrete reason (e.g., specific TUI rendering model).

## 4) Binary size concerns

two-face embeds assets (~0.6 MiB). Mitigations:

* Trust linker dead-code/asset elimination where applicable
* Only expose a small subset of themes in your UI/API
* Consider syntect-only if you truly need just a couple of languages

## 5) Theme choice mismatch for context

Some themes are intended for dark backgrounds; others for light. If you render HTML with an unexpected background, ensure your `<pre>` background matches the theme.

## 6) Licensing / attribution for embedded assets

two-face is MIT/Apache-2.0, but embedded assets can have their own licenses.

````rust
let text = two_face::acknowledgement::listing();
println!("{text}");
````

Include this in “About” pages or distributions if required by your policy.