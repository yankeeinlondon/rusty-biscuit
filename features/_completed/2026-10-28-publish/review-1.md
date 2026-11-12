---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Rollout records installation success without proving the required channels'
human_review: true
human_review_items:
  - "For later author review: Keep excluded manifest packages recognizable in approval files while leaving them ineligible for distribution."
  - "For later author review: Require approval at tagging and now; remove revoked channels, reject renamed channels, and reject revoked package approval."
  - "For later author review: Use the Cargo package name for binstall; reject alternate names that binstall cannot install."
  - "For later author review: Disable binstall compilation fallback in both channel states and validate the exact metadata."
  - "For later author review: Preserve existing help descriptions, use area README files where needed, and use repository area pages as homepages."
  - "For later author review: Prove reviewed releases through merged release-proposal pull requests whose final commit equals the tag commit."
  - "For later author review: Validate the base dist settings exactly, including GitHub mode, shell installers, and the Cargo home installation path."
  - "For later author review: Ignore foreign-repository release events and reject missing repository identity."
  - "For later author review: Set the Linux glibc floor in workspace configuration because the pinned dist ignores the package-level setting."
  - "For later author review: Generate source links and license notices in each staged package and reject collisions with checked-in files."
  - "For later author review: Allow only the intended command through a wildcard binary override covering every target."
  - "For later author review: Use the platform checksum verifier against dist-generated checksums rather than introduce new hashing code."
  - "For later author review: Prove lockfile consistency with locked metadata and before-and-after comparison because dist does not forward --locked."
  - "For later author review: Name the inspection helper inspect_archive.py to avoid shadowing a Python standard module."
  - "For later author review: Install Python as build-container tooling, separate from native build dependencies."
  - "For later author review: Check host-path leakage in library-loading metadata rather than embedded panic-message source paths."
  - "For later author review: Refuse unsupported installer platforms, explain refusals even in quiet mode, and validate the generated glibc checks."
  - "For later author review: Supplement license-tool output with package headers, omitted linked crates, and native-library notices; declare missing first-party licensing in about.toml."
  - "For later author review: Take nonpublishing verification approvals from dispatch inputs and independently prevent those selections from publishing."
  - "For later author review: Authenticate reused bundles against their original workflow and successful packaging job; refuse rebuilding over existing release assets."
  - "For later author review: Check bundle checksums before upload and compare existing release assets byte for byte, beyond bundle size checks."
  - "For later author review: Serialize active uploads by package and tap updates globally; manually recover superseded pending jobs."
  - "For later author review: Check the published shell installer in isolated directories on all four targets before publishing a formula."
  - "For later author review: Add platform floors and the claudine ALSA dependency to formulas; tolerate only measured upstream-template audit findings."
  - "For later author review: Leave darkmatter and claudine description changes to their Homebrew-enabling proposals because they also affect help text."
  - "For later author review: Verify tap commit signatures and token-owner identity using a pinned GitHub signing key; require credentials for dry runs."
  - "For later author review: Upload through the GitHub API, write provenance last, and download public assets without forwarding credentials."
  - "For later author review: Pin release tools by version and checksum and pass the selected binstall executable explicitly."
  - "For later author review: Disable binstall credential discovery, permit HTTP only for local fixtures, and check the actual downloaded URL and installed version."
  - "For later author review: Run Python checks as the Linux repo-deps companion suite and retain separate Rust workflow contracts."
  - "For later author review: Run existing repo-deps checks for distribution configuration changes rather than add a new companion-only scheduling mechanism."
  - "For later author review: Fail the distribution suite on skipped tests and require its local tools explicitly."
  - "For later author review: Require flat archive paths and simple release-asset filenames; stop unsafe archive inspection before native tools; share strict loading where possible, reject missing platform configuration, and use internal testing seams."
  - "For later author review: Verify prebuilt artifacts without rebuilding; pin the old-glibc test image separately, use offline installer downloads, observe macOS runtime libraries, and report emulation as pending."
  - "For later author review: Gate wider rollout on the current ledger and a successful first-package workflow; allow nonpublishing verification and account for recovery runs having a different workflow commit."
  - "For later author review: Use a strict rollout ledger with explicit target and channel results; retain the distinction between an unchanged older formula and an update in the tap report."
  - "For later author review: Produce a read-only rollout candidate for a later reviewed ledger change rather than updating the ledger automatically."
  - "For later author review: Permit one nonexecutable package changelog because dist includes existing changelogs automatically."
  - "For later author review: Use scoped Rust, Python, shell, and workflow checks instead of the workspace-wide lint recipe."
  - "For later author review: Bound license-generation Cargo commands at 30 minutes to accommodate cold registry access."
  - "For later author review: Cache successful policy parses only in tests; invalid cases still parse independently."
  - "For later author review: Leave live approval, release-App setup, tap setup, credentials, and the first real release pending; do not publish without authorization."
  - "For later author review: Leave wider rollout pending until the first bh chain and each later package have their required approval and installation evidence."
  - "For later author review: Leave unavailable clean macOS, native x86_64, and WSL2 runtime evidence pending rather than count emulation or a developer host as clean-runtime proof."
  - "For later author review: Leave Windows artifacts and PowerShell installation disabled until the prerequisite Windows repairs are verified."
  - "For later author review: Keep concrete installation URLs unadvertised until a real release supplies tested URLs."
  - "For later author review: Leave content-policy manifest license fields unchanged because about.toml already supplies the required notices."
  - "R1-F1 fix: Only a bh ledger entry with archive, shell-installer, brew, and binstall all 'pass' unlocks wider rollout, so the opening bh release must enable both brew and binstall (alternative: accept channels bh does not ship as disabled)."
  - "R1-F1 fix: A tap that already holds this exact version counts as current, so a retry can still install and record brew 'pass'; a newer formula records 'unchanged' (alternative: every no-op is 'unchanged')."
  - "R1-F1 fix: Whether the tap holds this version is an explicit strict tap-report field, formula_current, rather than parsed from reason text or compared in workflow shell."
  - "R1-F1 fix: The Homebrew install proof is a tap.py install-check subcommand behind the existing Runner seam, rather than a standalone shell script."
  - "R1-F1 fix: The widening gate uses the newest qualifying bh entry, not the newest bh entry overall."
  - "R1-F1 fix: Homebrew/brew 7.0.12 is pinned by tag and commit in the brew-install job env and installed only on runners without Homebrew; runners that ship Homebrew use their own version."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-09T12:54:18-08:00
spec: 2026-10-28-publish/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/rusty-biscuit/features/2026-10-28-publish/log.md
description: 'A **feature** review of `2026-10-28-publish/spec.md`'
feature: 2026-10-28-publish/review-1.md
next: 2026-10-28-publish/review-2.md
---

# Publish review — iteration 1

The feature is **not production ready**. One high-priority finding concerns installation evidence and the decision to allow additional packages to ship. No finding needs a human ruling before repair.

## Findings

### R1-F1 (high): Rollout records installation success without proving the required channels

**Authority — spec:** “Verify binstall for that target and Homebrew on supported macOS/Linux targets where the formula is enabled” and “Prove the complete chain on `bh` first, then enable the remaining first-set commands.” The spec also requires the first real release's install results and tap update to be recorded before widening.

**Class:** installation evidence is inferred from a different operation, or from disabled channels, and then used to authorize wider publication.

In the distribution automation, [the Homebrew job](../../.github/workflows/distribute-binaries.yml:762) audits the formula, downloads its archives for byte comparison, and publishes the formula. It never runs `brew install` or runs the command installed through Homebrew. Nevertheless, [rollout_candidate](../../scripts/ci/distribution.py:2115), the function that creates the proposed release-verification record, converts that job's success into `brew: pass`. A shell installation does not prove the formula's installation rules or Linux/macOS platform branches work.

The same decision accepts the first bh release with both optional channels disabled. [check_rollout](../../scripts/ci/distribution.py:2053), which permits subsequent packages, accepts that record. Thus an archive-only first release can unlock the remaining packages before the specified first complete chain succeeds. Reviewing the ledger change is useful, but the generated record should accurately state what the run proved.

#### Bounded class sweep

The sweep followed each installation result through the workflow, candidate generation, ledger parsing, and widening decision. It covered archive, shell, binstall, and Homebrew routes, including disabled and prerelease outcomes.

| Child ID | Site | Shape tested or controlling invariant | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | Homebrew workflow and candidate generation | Existing bh selection with `brew: bh`; install and tap jobs successful, binstall disabled | Public candidate API emits `brew: pass` without any Homebrew installation evidence | Require successful Homebrew installation checks on supported targets before claiming pass |
| R1-F1.2 | First-chain candidate and widening gate | Existing bh selection with both channels disabled; shell install successful | Candidate records disabled channels; parsed ledger permits darkmatter rollout | Keep wider rollout blocked until the first required complete installation chain is proven |
| R1-F1.3 | Ledger parser and widening gate | Candidate from R1-F1.1 parsed as the first verified entry | Parser accepts it and the widening decision returns the bh entry | Incomplete channel evidence cannot qualify as the first complete chain |
| R1-F1.4 | Archive smoke route | Workflow invariant: every target smoke job must succeed before upload | Clean for this class: actual archived command is executed | Execute the archived command |
| R1-F1.5 | Shell installation route | Workflow invariant: every target install job must succeed before candidate generation | Clean for this class: downloaded installer executes in an isolated directory and installed command runs | Exercise actual shell installation |
| R1-F1.6 | Binstall route | Enabled channel requires every target's binstall job to succeed | Clean for this class: job invokes actual binstall without compilation fallback and checks installed version/URL | Exercise actual binary installation |
| R1-F1.7 | Local runtime verifier | Enabled Homebrew/binstall without live evidence | Clean: reports pending rather than pass | Preserve pending until evidence exists |
| R1-F1.8 | Prerelease formula decision | Explicit invariant: prereleases never replace the stable formula | Clean: records unchanged rather than claiming a new formula update | Preserve stable formula; do not manufacture installation proof |

**Reproduction:** ran the public `rollout_candidate`, `parse_rollout`, and `check_rollout` APIs with the existing `test_distribution.selected` fixture. With `selected(brew="bh")`, `install="success"`, `tap="success"`, and `binstall="skipped"`, the Homebrew result was `pass` and the widening gate accepted the bh entry. With `selected()` and both channel jobs skipped, the result was `{archive: pass, shell-installer: pass, brew: disabled, binstall: disabled}` and the widening gate again accepted it. No network publication or synthetic malformed input was needed. Existing `RolloutCandidateTests.test_channel_results_follow_the_selection` explicitly expects the unsupported Homebrew pass.

**Recommended repair:** add bounded, isolated Homebrew installation checks for the supported formula targets and retain their results. Feed those results into candidate generation separately from formula publication. Require the first bh record used to unlock later packages to prove the required binstall and Homebrew chain; disabled channels remain valid for individual releases but do not prove the first complete chain. A retried older formula may remain unchanged, but its publication-job success alone proves no installation. Keep unavailable evidence pending. Update the candidate tests, workflow contract tests, and distribution guide together. This is within the existing spec and needs no new approval or live release during repair.

## Verification and coverage

`just distribution-test` passed: **432 tests in 19.366 seconds**, no skipped tests. The two targeted public-API reproductions above also succeeded and exposed the incorrect qualification. I read the real dist plan/build evidence retained in the implementation log, the release-selection and immutable-upload paths, archive/linkage inspection, installer checks, binstall checks, formula publisher, and rollout consumers. No release was published and no approval was changed during review.

| User-facing requirement | Strongest verification inspected | Assessment |
|---|---|---|
| Exact package/tag/revision, approval and channel selection | L1 public API and CLI fixtures; retained real dist plans for bh and md | Appropriate for the decision logic |
| Auxiliary executable exclusion, archive layout, source/license files | L1 archive tests plus recorded real build/plan evidence | Appropriate; later package rollout remains gated |
| Retry immutability, conflicting assets, no downgrade | L1 fake-service effects and byte comparisons | Appropriate for decision logic; real publication remains pending |
| Linux glibc floor and dependency inspection | L1 output fixtures plus recorded native arm64 Linux archive/runtime evidence | Remaining platform evidence stays pending, not an extra cross-OS readiness finding |
| Shell installer refusal and installation | L1 process fixtures, recorded real native installs, four-target workflow checks | Appropriate execution checks; pending hosts remain explicit |
| Binstall extraction and package-tag URL | L1 tests and retained real pinned-tool local-server acceptance; production jobs verify tagged manifests | Live Git-route and release acceptance remain pending |
| Homebrew installation | Formula audit/style and publication only | Wrong verification boundary: R1-F1 |
| Terminal interaction | No keyboard, focus, scrolling, or interactive UI requirement introduced | No L2/L3 keyboard or rendering proof required for this feature |

The review does not treat absent approvals, credentials, live publication, or cross-OS results as an additional finding. The spec explicitly calls for pending reporting when setup is absent. The defect above is that the implemented automation can later report an unproven channel as passed and use incomplete proof to widen distribution.

## Input robustness matrix

The configuration boundary uses strict YAML, JSON, and TOML loaders and shared field validators. The default is a located error for invalid shapes before publication. Deliberate exceptions include absent approval defaulting false, empty policy/ledger lists granting no new authority, disabled channel values, and documented optional fields.

I inspected and ran the shared-validator matrices for absent, null, wrong whole-field type, mixed/all-invalid collection elements, and empty values; the per-field wiring checks; duplicate/trailing-content loader checks; and CLI location checks. The independent build-image inspection loader has its own matrix. Policy, workspace metadata, dist settings/plans, selections, provenance, bundle metadata, rollout records, and service-response readers have control and invalid-shape checks in the passing suite. Native notices and tap-response validators have dedicated nullable-value checks. A search of changed Rust workflow contracts found defaults in test inspection helpers, not new production configuration deserializers. The main production configuration logic is Python; its explicit validators distinguish missing from null rather than filtering invalid entries. No additional realistic malformed-input defect was established by this review.

## Implementation decisions (informational)

Two independent agents read the implementation log and spec in parallel. The spec authorizes generated versus checked opt-ins, explicit uploads when needed, binstall metadata when discovery needs it, and reporting unavailable setup/live evidence as pending. The Linux floor was already ruled by the author. None of the decisions below requires a new human ruling: the recommended choices remain within the spec, are internal and reversible, or preserve an existing author decision. Reporting an item here does not waive R1-F1.

1. Keep excluded manifest packages recognizable in approval files while leaving them ineligible for distribution.
2. Require approval at tagging and now; remove revoked channels, reject renamed channels, and reject revoked package approval.
3. Use the Cargo package name for binstall; reject alternate names that binstall cannot install.
4. Disable binstall compilation fallback in both channel states and validate the exact metadata.
5. Preserve existing help descriptions, use area README files where needed, and use repository area pages as homepages.
6. Prove reviewed releases through merged release-proposal pull requests whose final commit equals the tag commit.
7. Validate the base dist settings exactly, including GitHub mode, shell installers, and the Cargo home installation path.
8. Ignore foreign-repository release events and reject missing repository identity.
9. Set the Linux glibc floor in workspace configuration because the pinned dist ignores the package-level setting.
10. Generate source links and license notices in each staged package and reject collisions with checked-in files.
11. Allow only the intended command through a wildcard binary override covering every target.
12. Use the platform checksum verifier against dist-generated checksums rather than introduce new hashing code.
13. Prove lockfile consistency with locked metadata and before-and-after comparison because dist does not forward --locked.
14. Name the inspection helper inspect_archive.py to avoid shadowing a Python standard module.
15. Install Python as build-container tooling, separate from native build dependencies.
16. Check host-path leakage in library-loading metadata rather than embedded panic-message source paths.
17. Refuse unsupported installer platforms, explain refusals even in quiet mode, and validate the generated glibc checks.
18. Supplement license-tool output with package headers, omitted linked crates, and native-library notices; declare missing first-party licensing in about.toml.
19. Take nonpublishing verification approvals from dispatch inputs and independently prevent those selections from publishing.
20. Authenticate reused bundles against their original workflow and successful packaging job; refuse rebuilding over existing release assets.
21. Check bundle checksums before upload and compare existing release assets byte for byte, beyond bundle size checks.
22. Serialize active uploads by package and tap updates globally; manually recover superseded pending jobs.
23. Check the published shell installer in isolated directories on all four targets before publishing a formula.
24. Add platform floors and the claudine ALSA dependency to formulas; tolerate only measured upstream-template audit findings.
25. Leave darkmatter and claudine description changes to their Homebrew-enabling proposals because they also affect help text.
26. Verify tap commit signatures and token-owner identity using a pinned GitHub signing key; require credentials for dry runs.
27. Upload through the GitHub API, write provenance last, and download public assets without forwarding credentials.
28. Pin release tools by version and checksum and pass the selected binstall executable explicitly.
29. Disable binstall credential discovery, permit HTTP only for local fixtures, and check the actual downloaded URL and installed version.
30. Run Python checks as the Linux repo-deps companion suite and retain separate Rust workflow contracts.
31. Run existing repo-deps checks for distribution configuration changes rather than add a new companion-only scheduling mechanism.
32. Fail the distribution suite on skipped tests and require its local tools explicitly.
33. Require flat archive paths and simple release-asset filenames; stop unsafe archive inspection before native tools; share strict loading where possible, reject missing platform configuration, and use internal testing seams.
34. Verify prebuilt artifacts without rebuilding; pin the old-glibc test image separately, use offline installer downloads, observe macOS runtime libraries, and report emulation as pending.
35. Gate wider rollout on the current ledger and a successful first-package workflow; allow nonpublishing verification and account for recovery runs having a different workflow commit.
36. Use a strict rollout ledger with explicit target and channel results; retain the distinction between an unchanged older formula and an update in the tap report.
37. Produce a read-only rollout candidate for a later reviewed ledger change rather than updating the ledger automatically.
38. Permit one nonexecutable package changelog because dist includes existing changelogs automatically.
39. Use scoped Rust, Python, shell, and workflow checks instead of the workspace-wide lint recipe.
40. Bound license-generation Cargo commands at 30 minutes to accommodate cold registry access.
41. Cache successful policy parses only in tests; invalid cases still parse independently.
42. Leave live approval, release-App setup, tap setup, credentials, and the first real release pending; do not publish without authorization.
43. Leave wider rollout pending until the first bh chain and each later package have their required approval and installation evidence.
44. Leave unavailable clean macOS, native x86_64, and WSL2 runtime evidence pending rather than count emulation or a developer host as clean-runtime proof.
45. Leave Windows artifacts and PowerShell installation disabled until the prerequisite Windows repairs are verified.
46. Keep concrete installation URLs unadvertised until a real release supplies tested URLs.
47. Leave content-policy manifest license fields unchanged because about.toml already supplies the required notices.

The summaries cover all 40 `decisions_for_review` groups in the implementation log, including their grouped subchoices, the test-only parse cache recorded in the phase log, and all reported deferrals. The license declaration workaround is sufficient for this feature; manifest cleanup need not block distribution. Description edits can accompany the later packages' enabling proposals, whose audits already refuse invalid descriptions. Unavailable runtime evidence remains pending. Windows and additional package rollout retain their existing gates. No work is moved to a new specification by this review.

The author may confirm or reverse these choices after the fix cycle. No response is needed to repair the finding, and no item is reserved for an author decision.
