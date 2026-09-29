# Spike: `FileChanged` Path Resolution

Date: 2026-09-28. Run on macOS; Windows behavior reasoned from
`#[cfg(windows)]` code and tests. Throwaway code: `src/main.rs` in this
directory (depends on `biscuit-file` with only the `file-reference` feature).

## Question

Can Biscuit File's `FileReference` resolve a `FileChanged(<path>, @prop)`
path against an explicit base directory, with the same result on macOS,
Linux, and Windows, and can it separate "file missing" from "cannot read"?

## Verdict

Yes, as a thin wrapper (about 20 lines), provided content-policy restricts the
accepted path forms first. Build the context with
`FileResolutionContext::from_snapshot(base, None, HashMap::new())` (no
repository root) and call `resolve_detailed`. That gives one probe relative to
the base, no process-state reads, and a clean missing/unreadable split.

## Observed Resolution (base `<tmp>/repo/docs`)

| Input | Result |
| --- | --- |
| `src/config.rs` | `base/src/config.rs`. With a repository root in the context, a miss at the base silently falls back to `<repo>/src/config.rs` |
| `./x`, `../x`, `../../outside` | One candidate each, normalized lexically. No containment check: `../../outside` escapes base and repository |
| `/etc/hosts` | Absolute; matched. On Windows it means the current drive's root |
| `C:\x`, `C:/x` on POSIX | Classified absolute, then probed as a nonexistent relative name: silent "missing", which would read as "source removed" |
| `a\b` on POSIX | Matches a file literally named `a\b`; on Windows it is `a/b`. OS-dependent |
| `C:x`, `x:y` | Parse error (`UnsupportedScheme`) |
| `~/x` | `<home>/x`, or `MissingHomeContext` without a home |
| `@x` | Search chain: base, repository, package roots, home |
| `&x`, `^x` | `MissingContext` without a repository |
| `%x` | Recursive search under the base |
| `vault:x` | `VaultNotConfigured` |
| `{{HOME}}` | Expanded from the environment |
| Relative base directory | Probed against the process's current directory, so the base must be absolute |

## Separators, Symlinks, Case

- On Windows, `base.join("src/config.rs")` works and matches are rewritten
  with `\` (covered by existing `#[cfg(windows)]` tests).
- Paths are never canonicalized (`/var/...` stays, not `/private/var/...`).
- Existence checks follow symlinks; a broken symlink counts as missing.
- File-name case matches on macOS and Windows, not on Linux.

## Missing Versus Unreadable

| Situation | `FileReference` result | Policy result |
| --- | --- | --- |
| File absent | `NoMatch`, candidate `Missing` | `triggered` (source removed) |
| Path is a directory | `NoMatch`, `NonFile` | `unknown` (recommended) |
| Parent unreadable | `Io(PermissionDenied)` | `unknown` |
| Hash-time read error | not-found vs other I/O | removed vs `unknown` |

## Grammar Constraints (YAML checked with PyYAML; indicative for serde_yaml_ng)

- ` #` in an unquoted rule starts a comment and truncates it to
  `FileChanged(a`, which then fails validation.
- `: ` turns the list item into a mapping, which is a shape error.
- Quoting the whole rule string fixes both.
- `,` and `)` in a path conflict with the compact grammar.
- The schema's illustrative pattern still admits leading/trailing spaces, `\`,
  `@`, `~`, `%`, and `{{`.

## Recommended Rulings

1. **Escaping the base with `..`: allow, without enforcement.** Documents
   legitimately watch `../src`; only a digest is stored. Cost: any readable
   file can be fingerprinted. Enforcing a boundary would need new code (the
   containment check is crate-private).
2. **Absolute paths: reject.** They are not portable in either direction.
3. **Sigils and machine-dependent forms (`@`, `~`, `&`, `^`, `%`, `vault:`,
   `{{VAR}}`, URLs, `\`): reject.** Never set a repository root in the
   context (or rewrite bare paths to `./…`) so no fallback applies.
4. **`,` and `)`: unsupported.** Document quoting the whole rule for ` #` and
   `: `; reject leading/trailing whitespace.

## Consequence for the Plan

The phase-2 path layer is a thin wrapper: validate the path form with
`FileReference::class()`, require an absolute base, build the snapshot context
without a repository, call `resolve_detailed`, and map the outcome to present,
removed, or unknown. No new normalization code.
