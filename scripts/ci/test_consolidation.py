#!/usr/bin/env python3
"""Contract tests for `scripts/ci/consolidation.py`, the consolidation toolkit.

Every guard is pinned twice: a green fixture that the guard accepts and the
smallest red fixture it must refuse (missing mapping, drifted attribute,
identity loss masked by identity gain, silent count match, …). The synthetic
fixtures carry their own filter strings. The shipped-artifact tests read the
real `_tier_filter`, `.config/nextest.toml`, and the Phase 1 baseline of
`2026-09-21-consolidated-test-binaries`, and skip when those are unavailable.
"""

from __future__ import annotations

import contextlib
import copy
import gzip
import io
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import consolidation as tool  # noqa: E402

REPO = tool.REPO_ROOT
JUST = shutil.which("just")

#: Fixture copies shaped like the canonical expressions. Production code never
#: holds these; it asks `just _tier_filter`.
FIXTURE_FILTERS = {
    "L1": "!(test(/(^|::)level2_/) + test(/(^|::)level3_/) + test(/(^|::)browser_/) + test(/(^|::)real_/) + test(/(^|::)slow_/))",
    "L2": "test(/(^|::)level2_/)",
    "L3": "test(/(^|::)level3_/)",
    "override-default-0": "test(=rate_limit_waits)",
    "override-ci-0": "test(/level2_/)",
}


def baseline_dir() -> Path | None:
    found = sorted((REPO / "features").glob("**/2026-09-21-consolidated-test-binaries/baseline"))
    return found[0] if found and (found[0] / "listings").is_dir() else None


def raw_listing(package: str, suites: dict[str, dict[str, str]], *, ignored: set[str] = frozenset(), cwd: str = "/a") -> dict:
    """A raw nextest listing; each test maps to `matches`, `expression`, or `ignored`."""
    rust_suites = {}
    for target, tests in suites.items():
        cases = {}
        for test, verdict in tests.items():
            match = {"status": "matches"} if verdict == "matches" else {"status": "mismatch", "reason": verdict}
            cases[test] = {"kind": "test", "ignored": verdict == "ignored" or test in ignored, "filter-match": match}
        rust_suites[f"{package}::{target}"] = {
            "package-name": package, "binary-id": f"{package}::{target}", "binary-name": target,
            "package-id": f"path+file://{cwd}#{package}@0.1.0", "kind": "test",
            "binary-path": f"{cwd}/target/debug/deps/{target}-abc", "build-platform": "target",
            "cwd": cwd, "status": "listed", "testcases": cases,
        }
    return {"rust-build-meta": {"target-directory": f"{cwd}/target"}, "test-count": 0, "rust-suites": rust_suites}


def evaluate_all(package: str, suites: dict[str, list[str]], filters: dict[str, str], ignored: set[str] = frozenset()) -> dict[str, dict]:
    """Per-selector raw listings whose verdicts come from the evaluator itself."""
    listings = {}
    for selector, text in filters.items():
        node = tool.parse_filter(text)
        verdicts = {}
        for target, tests in suites.items():
            verdicts[target] = {}
            for test in tests:
                if test in ignored:
                    verdicts[target][test] = "ignored"
                    continue
                subject = tool.FilterSubject(package, f"{package}::{target}", target, "test", test)
                verdicts[target][test] = "matches" if tool.evaluate_filter(node, subject) else "expression"
        listings[selector] = (text, raw_listing(package, verdicts))
    return listings


def make_capture(package: str, suites: dict[str, list[str]], *, host: str = "darwin", features: tuple = (),
                 filters: dict[str, str] = FIXTURE_FILTERS, ignored: set[str] = frozenset()) -> dict:
    return tool.build_capture(host=host, package=package, features=features,
                              listings=evaluate_all(package, suites, filters, ignored))


def captures_of(*documents: dict) -> dict:
    return {(d["host"], d["package"], d["feature_set"]): d for d in documents}


class FilterEvaluatorTests(unittest.TestCase):
    def verdict(self, text: str, test: str, package: str = "pkg", binary: str = "t") -> bool:
        return tool.evaluate_filter(tool.parse_filter(text), tool.FilterSubject(package, f"{package}::{binary}", binary, "test", test))

    def test_anchored_marker_matches_segment_start_only(self) -> None:
        self.assertTrue(self.verdict(FIXTURE_FILTERS["L2"], "level2_errors::plain"))
        self.assertTrue(self.verdict(FIXTURE_FILTERS["L2"], "mod::level2_case"))
        self.assertFalse(self.verdict(FIXTURE_FILTERS["L2"], "wrap_level2_case"))
        self.assertTrue(self.verdict("test(/level2_/)", "wrap_level2_case"), "an unanchored override regex matches mid-segment")

    def test_binary_name_is_never_the_test_path(self) -> None:
        self.assertTrue(self.verdict(FIXTURE_FILTERS["L1"], "plain", binary="level2_errors"))

    def test_precedence_and_difference(self) -> None:
        self.assertTrue(self.verdict("test(=a) | test(=b) & test(=c)", "a"), "& binds tighter than |")
        self.assertFalse(self.verdict("(test(=a) | test(=b)) & test(=c)", "a"))
        self.assertFalse(self.verdict("test(~x) - test(=xy)", "xy"))
        self.assertTrue(self.verdict("test(~x) - test(=xy)", "xz"))
        self.assertTrue(self.verdict("not test(=a) and all()", "b"))
        self.assertFalse(self.verdict("none() or !all()", "b"))

    def test_matchers_and_defaults(self) -> None:
        self.assertTrue(self.verdict("test(ompo)", "compose"), "test() defaults to contains")
        self.assertFalse(self.verdict("test(=ompo)", "compose"))
        self.assertTrue(self.verdict("test(#comp*)", "compose"))
        self.assertFalse(self.verdict("package(claudine)", "x", package="claudine-cli"), "package() defaults to glob, not contains")
        self.assertTrue(self.verdict("package(claudine-*)", "x", package="claudine-cli"))
        self.assertTrue(self.verdict("binary_id(pkg::t) & kind(test)", "x"))
        self.assertTrue(self.verdict(r"test(/a\/b/)", "a/b"))

    def test_unsupported_or_malformed_filters_are_refused(self) -> None:
        for text in ("platform(host)", "deps(x)", "test(", "test(=a) &", "(test(=a)", "test(/unterminated)"):
            with self.subTest(text=text), self.assertRaises(tool.FilterError):
                tool.parse_filter(text)


@unittest.skipUnless(JUST, "just is not installed")
class ShippedFilterCorpusTests(unittest.TestCase):
    """Every filter a capture or plan will read parses and evaluates."""

    def test_every_tier_and_override_filter_parses(self) -> None:
        filters = {tier: tool.tier_filter(tier, "claudine-cli") for tier in tool.TIER_SELECTORS}
        filters[tool.INCLUDE_SLOW_SELECTOR] = tool.tier_filter("L1", "darkmatter", include_slow=True)
        filters["worktree-L1"] = tool.tier_filter("L1", "worktree-cli")
        filters.update(tool.override_filters())
        self.assertTrue(any(s.startswith("override-") for s in filters), "nextest.toml overrides were read")
        for selector, text in filters.items():
            with self.subTest(selector=selector):
                node = tool.parse_filter(text)
                tool.evaluate_filter(node, tool.FilterSubject("claudine-cli", "claudine-cli::l1", "l1", "test", "m::t"))

    def test_include_slow_is_captured_only_where_the_manifest_declares_it(self) -> None:
        self.assertIn(tool.INCLUDE_SLOW_SELECTOR, tool.selector_filters("darkmatter", REPO / "darkmatter/lib/Cargo.toml"))
        self.assertNotIn(tool.INCLUDE_SLOW_SELECTOR, tool.selector_filters("claudine-cli", REPO / "claudine/cli/Cargo.toml"))


class CaptureDocumentTests(unittest.TestCase):
    def test_listings_fold_into_per_selector_verdicts(self) -> None:
        capture = make_capture("pkg", {"alpha": ["plain", "level2_case"]})
        tests = capture["suites"]["pkg::alpha"]["tests"]
        self.assertEqual({"L1": "matches", "L2": "expression"}, {k: tests["plain"]["verdicts"][k] for k in ("L1", "L2")})
        self.assertEqual("matches", tests["level2_case"]["verdicts"]["L2"])
        self.assertEqual("none", capture["feature_set"])

    def test_digest_ignores_locality_but_not_content(self) -> None:
        a = raw_listing("pkg", {"t": {"x": "matches"}}, cwd="/one")
        b = raw_listing("pkg", {"t": {"x": "matches"}}, cwd="/two")
        c = raw_listing("pkg", {"t": {"x": "expression"}}, cwd="/one")
        self.assertEqual(tool.listing_digest(a), tool.listing_digest(b))
        self.assertNotEqual(tool.listing_digest(a), tool.listing_digest(c))

    def test_listings_of_one_build_must_name_the_same_tests(self) -> None:
        listings = {"L1": (None, raw_listing("pkg", {"t": {"x": "matches"}})),
                    "L2": (None, raw_listing("pkg", {"t": {"x": "expression", "y": "matches"}}))}
        with self.assertRaisesRegex(tool.ToolError, "names different tests"):
            tool.build_capture(host="darwin", package="pkg", features=(), listings=listings)

    def test_a_binary_skipped_by_a_package_filter_takes_listed_mismatch_verdicts(self) -> None:
        # Observed on nextest 0.9.136: `package(claudine) & …` under `-p claudine-cli`
        # reports every suite `status: skipped` with no test cases.
        listed = raw_listing("pkg", {"t": {"x": "matches", "slow": "ignored"}})
        skipped = raw_listing("pkg", {"t": {}})
        skipped["rust-suites"]["pkg::t"]["status"] = "skipped"
        capture = tool.build_capture(host="darwin", package="pkg", features=(),
                                     listings={"L1": ("all()", listed), "override-ci-1": ("package(other)", skipped)})
        tests = capture["suites"]["pkg::t"]["tests"]
        self.assertEqual(("expression", "ignored"), (tests["x"]["verdicts"]["override-ci-1"], tests["slow"]["verdicts"]["override-ci-1"]))
        self.assertEqual(["pkg::t"], capture["selectors"]["override-ci-1"]["skipped_binaries"])
        with self.assertRaisesRegex(tool.ToolError, "skipped by every selector"):
            tool.build_capture(host="darwin", package="pkg", features=(), listings={"override-ci-1": ("package(other)", skipped)})
        missing = raw_listing("pkg", {})
        with self.assertRaisesRegex(tool.ToolError, "different binaries"):
            tool.build_capture(host="darwin", package="pkg", features=(), listings={"L1": (None, listed), "L2": (None, missing)})

    def test_round_trip_and_raw_listing_directories(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture = make_capture("pkg", {"t": ["x"]}, features=("a", "b"))
            tool.write_gzip_json(root / "darwin__pkg__a+b.capture.json.gz", capture)
            first = (root / "darwin__pkg__a+b.capture.json.gz").read_bytes()
            tool.write_gzip_json(root / "darwin__pkg__a+b.capture.json.gz", capture)
            self.assertEqual(first, (root / "darwin__pkg__a+b.capture.json.gz").read_bytes(), "gzip output is deterministic")
            raw = root / "raw"
            raw.mkdir()
            (raw / "linux__pkg__none__L1.json.gz").write_bytes(gzip.compress(json.dumps(raw_listing("pkg", {"t": {"x": "matches"}})).encode()))
            loaded = tool.load_captures([root / "darwin__pkg__a+b.capture.json.gz", raw])
            self.assertEqual(capture, loaded[("darwin", "pkg", "a+b")])
            self.assertEqual("matches", loaded[("linux", "pkg", "none")]["suites"]["pkg::t"]["tests"]["x"]["verdicts"]["L1"])

    def test_an_empty_listing_is_never_zero_tests(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "darwin__pkg__none__L1.json"
            path.write_text("")
            with self.assertRaisesRegex(tool.ToolError, "never read as zero tests"):
                tool.load_captures([path])


class InventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="consolidation-inventory-")
        self.repo = Path(self.temporary.name)
        self.crate = self.repo / "pkg"
        (self.crate / "tests").mkdir(parents=True)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def write(self, relative: str, text: str) -> Path:
        path = self.crate / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        return path

    def metadata(self, targets: dict[str, str], features: dict[str, list[str]] | None = None) -> dict:
        return {
            "workspace_members": ["pkg-id"],
            "packages": [{
                "id": "pkg-id", "name": "pkg", "manifest_path": str(self.crate / "Cargo.toml"),
                "targets": [{"name": name, "kind": ["test"], "src_path": str(self.crate / path),
                             "required-features": (features or {}).get(name, []), "edition": "2024"}
                            for name, path in targets.items()],
            }],
        }

    def inventory(self, metadata: dict, captures: dict | None = None) -> tuple[dict, list[str]]:
        return tool.build_inventory(metadata, self.repo, packages=["pkg"], captures=captures)

    def test_auto_discovered_and_declared_targets_merge(self) -> None:
        self.write("Cargo.toml", '[package]\nname = "pkg"\n[features]\nterm = []\n[[test]]\nname = "level2_x"\nrequired-features = ["term"]\n')
        self.write("tests/alpha.rs", "#![cfg(unix)]\nmod common;\n#[test]\nfn plain() {}\n")
        self.write("tests/level2_x.rs", "#[test]\nfn level2_y() {}\n")
        self.write("tests/nested/main.rs", "#[test]\nfn n() {}\n")
        metadata = self.metadata({"alpha": "tests/alpha.rs", "level2_x": "tests/level2_x.rs", "nested": "tests/nested/main.rs"},
                                 {"level2_x": ["term"]})
        capture = make_capture("pkg", {"alpha": ["plain"], "level2_x": ["level2_y"], "nested": ["n"]})
        inventory, failures = self.inventory(metadata, captures_of(capture))
        self.assertEqual([], failures)
        targets = {t["name"]: t for t in inventory["packages"]["pkg"]["test_targets"]}
        self.assertEqual(["unix"], targets["alpha"]["inner_cfg"])
        self.assertTrue(targets["alpha"]["declares_mod_common"])
        self.assertTrue(targets["nested"]["nested_root"])
        self.assertEqual(["term"], targets["level2_x"]["required_features"])
        self.assertEqual(1, targets["level2_x"]["selected_by_tier"]["none"]["L2"])

    def test_a_declared_target_cargo_does_not_report_fails(self) -> None:
        self.write("Cargo.toml", '[package]\nname = "pkg"\n[[test]]\nname = "ghost"\npath = "tests/ghost.rs"\n')
        _, failures = self.inventory(self.metadata({}))
        self.assertTrue(any("cargo metadata does not report it" in f for f in failures), failures)
        self.assertTrue(any("tests/ghost.rs does not exist" in f for f in failures), failures)

    def test_an_undeclared_target_under_autotests_false_fails(self) -> None:
        self.write("Cargo.toml", '[package]\nname = "pkg"\nautotests = false\n')
        self.write("tests/alpha.rs", "")
        _, failures = self.inventory(self.metadata({"alpha": "tests/alpha.rs"}))
        self.assertTrue(any("not declared" in f for f in failures), failures)

    def test_an_unbuilt_nested_crate_root_fails(self) -> None:
        self.write("Cargo.toml", '[package]\nname = "pkg"\nautotests = false\n[[test]]\nname = "l1"\npath = "tests/l1/main.rs"\n')
        self.write("tests/l1/main.rs", "")
        self.write("tests/orphan/main.rs", "")
        _, failures = self.inventory(self.metadata({"l1": "tests/l1/main.rs"}))
        self.assertEqual(1, len(failures), failures)
        self.assertIn("tests/orphan/main.rs is a nested crate root that no test target builds", failures[0])

    def test_captures_from_two_hosts_are_refused(self) -> None:
        self.write("Cargo.toml", '[package]\nname = "pkg"\n')
        captures = captures_of(make_capture("pkg", {"t": ["x"]}), make_capture("pkg", {"t": ["x"]}, host="linux"))
        with self.assertRaisesRegex(tool.ToolError, "one host"):
            self.inventory(self.metadata({}), captures)


def inventory_of(targets: list[dict]) -> dict:
    defaults = {"harness": True, "target_wide_keys": [], "required_features": [], "inner_cfg": [], "nested_root": False}
    rows = []
    for target in targets:
        row = {**defaults, **target}
        row.setdefault("src_path", f"pkg/tests/{row['name']}.rs")
        if row["required_features"] and "required-features" not in row["target_wide_keys"]:
            row["target_wide_keys"] = ["required-features", *row["target_wide_keys"]]
        rows.append(row)
    return {"packages": {"pkg": {"manifest": "pkg/Cargo.toml", "test_targets": rows}}}


class PlanTests(unittest.TestCase):
    def plan(self, targets: list[dict], suites: dict[str, list[str]], filters: dict[str, str] = FIXTURE_FILTERS,
             captures: dict | None = None) -> tool.PlanResult:
        captures = captures if captures is not None else captures_of(make_capture("pkg", suites, filters=filters))
        return tool.plan_package("pkg", inventory_of(targets), captures, filters, repo=Path(tempfile.gettempdir()))

    def modules(self, result: tool.PlanResult) -> dict[str, dict]:
        return {m["old_target"]: m for m in result.manifest["modules"]}

    def test_contracts_become_named_targets(self) -> None:
        result = self.plan(
            [{"name": "alpha"}, {"name": "level2_x", "required_features": ["terminal-tests"]},
             {"name": "level3_t", "required_features": ["terminal-tests"]}, {"name": "level3_b", "required_features": ["browser-tests"]},
             {"name": "nested", "nested_root": True, "src_path": "pkg/tests/nested/main.rs"}],
            {"alpha": ["plain"], "level2_x": ["level2_y"], "level3_t": ["level3_a"], "level3_b": ["level3_c"], "nested": ["inner::n"]},
        )
        self.assertEqual([], result.failures)
        targets = {t["name"]: t for t in result.manifest["targets"]}
        self.assertEqual({"l1", "level2", "level3-terminal", "level3-browser"}, set(targets))
        self.assertEqual(["alpha", "nested"], targets["l1"]["modules"])
        self.assertEqual("pkg/tests/level3-browser/main.rs", targets["level3-browser"]["path"])
        modules = self.modules(result)
        self.assertEqual("pkg/tests/l1/nested/mod.rs", modules["nested"]["new_path"])
        self.assertEqual("pkg/tests/l1/alpha.rs", modules["alpha"]["new_path"])
        self.assertEqual([{"old": "inner::n", "new": "nested::inner::n"}], modules["nested"]["tests"])
        self.assertIsNone(modules["level2_x"]["alias"])

    def test_a_marker_name_that_would_move_tests_gets_a_neutral_alias(self) -> None:
        # The darkmatter-cli shape: gated on terminal-tests, but its tests are L1.
        result = self.plan([{"name": "level2_harness_integrity", "required_features": ["terminal-tests"]},
                            {"name": "level2_other", "required_features": ["terminal-tests"]}],
                           {"level2_harness_integrity": ["accepts_valid_link"], "level2_other": ["level2_case"]})
        self.assertEqual([], result.failures)
        module = self.modules(result)["level2_harness_integrity"]
        self.assertEqual("harness_integrity", module["module"])
        self.assertEqual("level2_harness_integrity", module["alias"]["from"])
        self.assertTrue(any(reason.startswith("L1:") for reason in module["alias"]["reason"]))
        self.assertEqual([{"old": "accepts_valid_link", "new": "harness_integrity::accepts_valid_link"}], module["tests"])

    def test_an_alias_that_collides_gets_a_module_suffix(self) -> None:
        result = self.plan([{"name": "level2_errors", "required_features": ["f"]}, {"name": "errors", "required_features": ["f"]}],
                           {"level2_errors": ["plain"], "errors": ["level2_x"]})
        # `errors` lives in the L1-named contract for `f`; the alias shares the level2 target only with itself.
        self.assertEqual([], result.failures)
        self.assertEqual("errors", self.modules(result)["level2_errors"]["module"])
        crowded = self.plan([{"name": "level2_common", "required_features": ["f"]}], {"level2_common": ["plain"]})
        self.assertEqual("common_module", self.modules(crowded)["level2_common"]["module"])

    def test_an_exact_name_override_is_a_rewrite_not_an_alias(self) -> None:
        result = self.plan([{"name": "loop_cli"}], {"loop_cli": ["rate_limit_waits", "other"]})
        self.assertEqual([], result.failures)
        self.assertEqual("loop_cli", self.modules(result)["loop_cli"]["module"])
        self.assertEqual([{
            "selector": "override-default-0", "filter": "test(=rate_limit_waits)",
            "tests": [{"old": "pkg::loop_cli rate_limit_waits", "new": "pkg::l1 loop_cli::rate_limit_waits"}],
            "suggested_filter": "test(=loop_cli::rate_limit_waits)",
        }], result.manifest["override_rewrites"])

    def test_an_unmarked_name_matched_by_an_unanchored_override_is_refused(self) -> None:
        result = self.plan([{"name": "wrap_level2_extras"}], {"wrap_level2_extras": ["plain"]})
        self.assertTrue(any("carries no marker prefix to strip" in f for f in result.failures), result.failures)

    def test_reserved_names_custom_harness_and_target_wide_keys_are_refused(self) -> None:
        self.assertTrue(any("collides with a crate-root module" in f for f in self.plan([{"name": "common"}], {"common": ["x"]}).failures))
        custom = self.plan([{"name": "bench_like", "harness": False, "target_wide_keys": ["harness"]}], {"bench_like": ["x"]})
        self.assertTrue(any("harness = false" in f for f in custom.failures))
        edition = self.plan([{"name": "old", "target_wide_keys": ["edition"]}], {"old": ["x"]})
        self.assertTrue(any("sets edition" in f for f in edition.failures))

    def test_the_evaluator_must_agree_with_every_nextest_verdict(self) -> None:
        capture = make_capture("pkg", {"alpha": ["plain"]})
        green = self.plan([{"name": "alpha"}], {}, captures=captures_of(capture))
        self.assertEqual([], green.failures)
        red = copy.deepcopy(capture)
        red["suites"]["pkg::alpha"]["tests"]["plain"]["verdicts"]["L2"] = "matches"
        result = self.plan([{"name": "alpha"}], {}, captures=captures_of(red))
        self.assertTrue(any("evaluator disagrees with nextest" in f for f in result.failures), result.failures)

    def test_a_capture_taken_under_another_filter_is_refused(self) -> None:
        drifted = dict(FIXTURE_FILTERS, L2="test(/(^|::)level2x_/)")
        capture = make_capture("pkg", {"alpha": ["plain"]}, filters=drifted)
        result = self.plan([{"name": "alpha"}], {}, captures=captures_of(capture))
        self.assertTrue(any("recapture before planning" in f for f in result.failures), result.failures)

    def test_a_target_no_capture_lists_falls_back_to_a_source_scan(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "pkg/tests").mkdir(parents=True)
            (root / "pkg/tests/level3_win.rs").write_text("#![cfg(windows)]\n#[test]\nfn level3_ctrl_c() {}\n")
            result = tool.plan_package("pkg", inventory_of([{"name": "level3_win", "required_features": ["t"]}]),
                                       captures_of(make_capture("pkg", {})), FIXTURE_FILTERS, repo=root)
            module = result.manifest["modules"][0]
            self.assertEqual(("source-scan", "level3_win"), (module["test_basis"], module["module"]))
            self.assertEqual([{"old": "level3_ctrl_c", "new": "level3_win::level3_ctrl_c"}], module["tests"])


def manifest_for(package: str, modules: list[tuple[str, str, str]], additions: list[dict] | None = None) -> dict:
    """(old target, consolidated target, module) rows."""
    return {
        "package": package,
        "modules": [{"old_target": old, "old_binary_id": f"{package}::{old}", "target": target,
                     "binary_id": f"{package}::{target}", "module": module} for old, target, module in modules],
        "additions": additions or [],
    }


class CompareTests(unittest.TestCase):
    BEFORE = {"alpha": ["plain", "level2_marked"], "beta": ["plain"]}
    AFTER = {"l1": ["alpha::plain", "alpha::level2_marked", "beta::plain"]}
    MANIFEST = manifest_for("pkg", [("alpha", "l1", "alpha"), ("beta", "l1", "beta")])

    def compare(self, before: list[dict], after: list[dict], manifest: dict | None = MANIFEST, **options) -> dict:
        manifests = {"pkg": manifest} if manifest else {}
        return tool.compare_captures(captures_of(*before), captures_of(*after), manifests, **options)

    def test_an_unmigrated_tree_compares_identical_to_itself(self) -> None:
        capture = make_capture("pkg", self.BEFORE)
        report = self.compare([capture], [copy.deepcopy(capture)], manifest=None, require_identical_digests=True)
        self.assertEqual(("identical", []), (report["verdict"], report["failures"]))

    def test_a_consolidated_tree_normalizes_to_the_before_identities(self) -> None:
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", self.AFTER)])
        self.assertEqual([], report["failures"])
        selected = report["packages"]["pkg"]["cells"][0]["selectors"]["L2"]["sets"]["selected"]
        self.assertEqual({"before": 1, "after": 1, "lost": [], "gained": []}, selected)
        self.assertFalse(report["packages"]["pkg"]["cells"][0]["selectors"]["L2"]["listing_digest_identical"])

    def test_identity_loss_masked_by_identity_gain_fails_with_the_identities(self) -> None:
        # Same counts on every set: a count comparison would pass.
        after = {"l1": ["alpha::plain", "alpha::level2_marked", "beta::renamed"]}
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", after)])
        self.assertEqual("different", report["verdict"])
        present = report["packages"]["pkg"]["cells"][0]["selectors"]["L1"]["sets"]["present"]
        self.assertEqual((present["before"], present["after"]), (3, 3))
        self.assertEqual((["pkg::beta plain"], ["pkg::beta renamed"]), (present["lost"], present["gained"]))
        self.assertTrue(any("pkg::beta plain" in f for f in report["failures"]))

    def test_a_tier_move_by_module_name_fails(self) -> None:
        manifest = manifest_for("pkg", [("alpha", "l1", "alpha"), ("beta", "l1", "level2_beta")])
        after = {"l1": ["alpha::plain", "alpha::level2_marked", "level2_beta::plain"]}
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", after)], manifest=manifest)
        self.assertTrue(any("L2 selected differs" in f and "pkg::beta plain" in f for f in report["failures"]), report["failures"])

    def test_an_ignore_flip_fails(self) -> None:
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", self.AFTER, ignored={"beta::plain"})])
        self.assertTrue(any("L1 ignored differs" in f for f in report["failures"]), report["failures"])

    def test_an_unmapped_module_and_a_duplicate_normalization_fail(self) -> None:
        after = {"l1": ["alpha::plain", "alpha::level2_marked", "beta::plain", "stray::x"]}
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", after)])
        self.assertTrue(any("stray::x is not mapped" in f for f in report["failures"]), report["failures"])
        doubled = manifest_for("pkg", [("alpha", "l1", "alpha"), ("alpha", "l1", "alpha_again"), ("beta", "l1", "beta")])
        after = {"l1": ["alpha::plain", "alpha::level2_marked", "alpha_again::plain", "beta::plain"]}
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", after)], manifest=doubled)
        self.assertTrue(any("normalize to the same identity" in f for f in report["failures"]), report["failures"])

    def test_an_absent_consolidated_suite_is_lost_identities_not_zero(self) -> None:
        before = [make_capture("pkg", self.BEFORE), make_capture("pkg", {"level2_x": ["level2_y"]}, features=("t",))]
        manifest = manifest_for("pkg", [("alpha", "l1", "alpha"), ("beta", "l1", "beta"), ("level2_x", "level2", "level2_x")])
        after = [make_capture("pkg", self.AFTER), make_capture("pkg", {}, features=("t",))]
        report = self.compare(before, after, manifest=manifest)
        self.assertTrue(any("darwin/pkg/t: L2 selected differs — lost 1 ['pkg::level2_x level2_y']" in f for f in report["failures"]), report["failures"])

    def test_missing_captures_and_one_sided_selectors_fail(self) -> None:
        before = [make_capture("pkg", self.BEFORE), make_capture("pkg", self.BEFORE, features=("t",))]
        report = self.compare(before, [make_capture("pkg", self.AFTER)])
        self.assertTrue(any("darwin/pkg/t: before capture has no after capture" in f for f in report["failures"]))
        narrow = {k: v for k, v in FIXTURE_FILTERS.items() if k != "L3"}
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", self.AFTER, filters=narrow)])
        self.assertTrue(any("selector L3 was captured on the before side only" in f for f in report["failures"]))
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", self.AFTER, filters=narrow)], common_selectors=True)
        self.assertEqual([], report["failures"])
        self.assertTrue(any("--common-selectors" in n for n in report["notes"]))

    def test_a_tier_filter_change_fails_and_an_override_rewrite_is_noted(self) -> None:
        rewritten = dict(FIXTURE_FILTERS, **{"override-default-0": "test(=beta::plain)"})
        before = make_capture("pkg", {"alpha": [], "beta": ["plain"]}, filters=dict(FIXTURE_FILTERS, **{"override-default-0": "test(=plain)"}))
        manifest = manifest_for("pkg", [("alpha", "l1", "alpha"), ("beta", "l1", "beta")])
        report = self.compare([before], [make_capture("pkg", {"l1": ["beta::plain"]}, filters=rewritten)], manifest=manifest)
        self.assertEqual([], report["failures"])
        self.assertTrue(any("override override-default-0 was rewritten" in n for n in report["notes"]))
        unrewritten = make_capture("pkg", {"l1": ["beta::plain"]}, filters=dict(FIXTURE_FILTERS, **{"override-default-0": "test(=plain)"}))
        report = self.compare([before], [unrewritten], manifest=manifest)
        self.assertTrue(any("override-default-0 selected differs" in f for f in report["failures"]), "an unrewritten exact override stops matching")
        drifted = make_capture("pkg", self.AFTER, filters=dict(FIXTURE_FILTERS, L1="all()"))
        report = self.compare([make_capture("pkg", self.BEFORE)], [drifted])
        self.assertTrue(any("tier selector L1 changed its filter" in f for f in report["failures"]))

    def test_platform_absent_needs_other_hosts(self) -> None:
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", self.AFTER)])
        self.assertEqual("not-evaluated", report["packages"]["pkg"]["platform_absent"]["none"]["status"])
        linux_before = make_capture("pkg", {"alpha": ["plain", "level2_marked"], "beta": ["plain", "linux_only"]}, host="linux")
        linux_after = make_capture("pkg", {"l1": ["alpha::plain", "alpha::level2_marked", "beta::plain", "beta::linux_only"]}, host="linux")
        report = self.compare([make_capture("pkg", self.BEFORE), linux_before], [make_capture("pkg", self.AFTER), linux_after])
        self.assertEqual([], report["failures"])
        darwin = report["packages"]["pkg"]["platform_absent"]["none"]["per_host"]["darwin"]
        self.assertEqual((1, 1), (darwin["before"], darwin["after"]))
        # The Linux-only test now also compiles on macOS (a lost module cfg): its own set is equal, the absent set is not.
        leaked = make_capture("pkg", {"l1": self.AFTER["l1"] + ["beta::linux_only"]})
        report = self.compare([make_capture("pkg", self.BEFORE), linux_before], [leaked, linux_after])
        self.assertTrue(any("darwin/pkg/none: platform-absent differs" in f for f in report["failures"]), report["failures"])

    def test_declared_additions_are_excluded_and_must_exist(self) -> None:
        manifest = manifest_for("pkg", [("alpha", "l1", "alpha"), ("beta", "l1", "beta")],
                                additions=[{"target": "l1", "test": "beta::placement_guard"}])
        after = {"l1": self.AFTER["l1"] + ["beta::placement_guard"]}
        self.assertEqual([], self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", after)], manifest=manifest)["failures"])
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", self.AFTER)], manifest=manifest)
        self.assertTrue(any("declares addition" in f for f in report["failures"]))

    def test_required_digests_fail_on_any_listing_change(self) -> None:
        report = self.compare([make_capture("pkg", self.BEFORE)], [make_capture("pkg", self.AFTER)], require_identical_digests=True)
        self.assertTrue(any("listing digest differs" in f for f in report["failures"]))

    def test_cli_exit_codes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary, contextlib.redirect_stderr(io.StringIO()):
            root = Path(temporary)
            tool.write_gzip_json(root / "b.capture.json.gz", make_capture("pkg", self.BEFORE))
            tool.write_gzip_json(root / "a.capture.json.gz", make_capture("pkg", {"alpha": ["plain"], "beta": ["plain"]}))
            (root / "out").mkdir()
            self.assertEqual(0, tool.main(["compare", "--before", str(root / "b.capture.json.gz"), "--after", str(root / "b.capture.json.gz")]))
            self.assertEqual(1, tool.main(["compare", "--before", str(root / "b.capture.json.gz"), "--after", str(root / "a.capture.json.gz"),
                                           "--markdown", str(root / "out/report.md")]))
            self.assertIn("# Identity comparison: different", (root / "out/report.md").read_text())
            self.assertEqual(2, tool.main(["compare", "--before", str(root / "missing"), "--after", str(root / "b.capture.json.gz")]))


class SourceScanTests(unittest.TestCase):
    def test_inner_attributes_below_docs_and_across_lines(self) -> None:
        text = (
            "//! Long module doc mentioning #![cfg(fake)] in prose.\n/* block /* nested */ comment */\n"
            "#![allow(dead_code)]\n#![cfg(any(\n    target_os = \"linux\",\n    target_os = \"macos\"\n))]\n"
            "#![doc = \"a ] bracket\"]\nuse std::fs;\n#![cfg(never)]\n"
        )
        self.assertEqual(['allow(dead_code)', 'cfg(any(target_os="linux",target_os="macos"))', 'doc="a ] bracket"'],
                         tool.leading_inner_attributes(text))

    def test_module_declarations_with_attributes(self) -> None:
        text = (
            "#![allow(unused)]\n#[path = \"../common/mod.rs\"]\nmod common;\n/// doc\n#[cfg(unix)]\npub mod alpha;\n"
            "const X: &str = \"mod fake; }\";\nfn helper() { let _ = '{'; }\n#[cfg(\n  windows\n)]\nmod beta;\nmod gamma { mod inner; }\n"
        )
        self.assertEqual({"common": ['path="../common/mod.rs"'], "alpha": ["cfg(unix)"], "beta": ["cfg(windows)"]},
                         tool.module_declarations(text))


class CheckAttributesTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="consolidation-attributes-")
        root = Path(self.temporary.name)
        self.before, self.repo = root / "before", root / "after"
        self.manifest = {
            "package": "pkg",
            "targets": [{"name": "l1", "path": "pkg/tests/l1/main.rs"}],
            "modules": [{"old_target": "alpha", "old_path": "pkg/tests/alpha.rs", "target": "l1", "module": "alpha",
                         "new_path": "pkg/tests/l1/alpha.rs", "nested_root": False}],
            "dispositions": [],
        }

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def write(self, root: Path, relative: str, text: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def check(self, former: str, moved: str, root: str) -> dict:
        self.write(self.before, "pkg/tests/alpha.rs", former)
        self.write(self.repo, "pkg/tests/l1/alpha.rs", moved)
        self.write(self.repo, "pkg/tests/l1/main.rs", root)
        return tool.check_attributes(self.manifest, tool.SourceReader(root=self.before), repo=self.repo)

    FORMER = "//! Windows-only capture.\n//!\n//! More docs.\n#![cfg(windows)]\n#![allow(dead_code)]\n#[test]\nfn plain() {}\n"
    MOVED = "//! Windows-only capture.\n//!\n//! More docs.\n#![allow(dead_code)]\n#[test]\nfn plain() {}\n"

    def test_a_cfg_moved_to_the_declaration_passes(self) -> None:
        report = self.check(self.FORMER, self.MOVED, "#[cfg(windows)]\nmod alpha;\n")
        self.assertEqual([], report["failures"])

    def test_a_cfg_missing_from_the_declaration_fails(self) -> None:
        report = self.check(self.FORMER, self.FORMER, "mod alpha;\n")
        self.assertEqual(1, len(report["failures"]), report["failures"])
        self.assertIn("lacks #[cfg(windows)]", report["failures"][0])

    def test_a_dropped_lint_attribute_fails(self) -> None:
        report = self.check(self.FORMER, "#[test]\nfn plain() {}\n", "#[cfg(windows)]\nmod alpha;\n")
        self.assertTrue(any("#![allow(dead_code)]" in f for f in report["failures"]), report["failures"])
        self.assertEqual([], self.check(self.FORMER, "#[test]\nfn plain() {}\n", "#[cfg(windows)]\n#[allow(dead_code)]\nmod alpha;\n")["failures"])

    def test_crate_root_only_attributes_fail(self) -> None:
        report = self.check("#![recursion_limit = \"256\"]\n", "#![recursion_limit = \"256\"]\n", "mod alpha;\n")
        self.assertEqual(2, len(report["failures"]), report["failures"])

    def test_a_missing_declaration_fails(self) -> None:
        self.assertTrue(any("does not declare `mod alpha;`" in f for f in self.check("", "", "mod beta;\n")["failures"]))

    def test_crate_global_and_identity_constructs_need_dispositions(self) -> None:
        moved = ('#[unsafe(no_mangle)]\npub extern "C" fn hook() {}\n'
                 'fn probe() { let _ = format!("{} --exact public_entry_points::probe", exe); }\n'
                 'fn listing() { std::env::current_exe(); let _ = "--list"; }\n')
        report = self.check("", moved, "mod alpha;\n")
        found = {f.split(": ")[1].split(" ")[0] for f in report["failures"]}
        self.assertEqual({"crate-global", "identity-sensitive"}, found, report["failures"])
        self.assertEqual(4, len(report["failures"]), report["failures"])
        self.manifest["dispositions"] = [{"path": "pkg/tests/l1/alpha.rs", "detector": d, "reason": "fixture"}
                                         for d in ("no_mangle", "extern_c_fn", "exact_path_string", "current_exe_list")]
        report = self.check("", moved, "mod alpha;\n")
        self.assertEqual([], report["failures"])
        self.assertEqual(2, sum(1 for r in report["review"] if r["kind"] == "identity-dispositioned"))

    def test_a_former_crate_path_fails(self) -> None:
        report = self.check("// crate:: in a comment is fine\nuse crate::helpers;\n", "", "mod alpha;\n")
        self.assertTrue(any("uses `crate::`" in f for f in report["failures"]), report["failures"])
        self.assertEqual([], self.check("// crate:: in a comment is fine\n", "", "mod alpha;\n")["failures"])

    def test_path_sensitive_constructs_are_reported_for_review(self) -> None:
        report = self.check("", '#[path = "x.rs"]\nmod x;\nconst S: &str = include_str!("f.txt");\n', "mod alpha;\n")
        self.assertEqual([], report["failures"])
        kinds = {r["detector"] for r in report["review"] if r["kind"] == "path-sensitive"}
        self.assertEqual({"mod_decl", "path_attr", "include_macro"}, kinds)


class CheckSnapshotsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="consolidation-snapshots-")
        root = Path(self.temporary.name)
        self.before, self.repo = root / "before", root / "after"
        self.manifest = {
            "package": "pkg", "crate_dir": "pkg",
            "modules": [
                {"old_target": "wrap_basics", "old_binary_id": "pkg::wrap_basics", "old_path": "pkg/tests/wrap_basics.rs",
                 "target": "l1", "module": "wrap_basics", "new_path": "pkg/tests/l1/wrap_basics.rs", "nested_root": False},
                {"old_target": "error_snapshots", "old_binary_id": "pkg::error_snapshots", "old_path": "pkg/tests/error_snapshots/main.rs",
                 "target": "l1", "module": "error_snapshots", "new_path": "pkg/tests/l1/error_snapshots/mod.rs", "nested_root": True},
                {"old_target": "level3_image", "old_binary_id": "pkg::level3_image", "old_path": "pkg/tests/level3_image.rs",
                 "target": "level3-terminal", "module": "level3_image", "new_path": "pkg/tests/level3-terminal/level3_image.rs", "nested_root": False},
            ],
        }
        self.files = {
            "pkg/tests/wrap_basics.rs": "#[test]\nfn help() { insta::assert_snapshot!(x); }\n",
            "pkg/tests/snapshots/wrap_basics__help.snap": "---\nsource: tests/wrap_basics.rs\n---\nhelp\n",
            "pkg/tests/snapshots/wrap_basics__nested__inner.snap": "inner\n",
            "pkg/tests/error_snapshots/main.rs": "mod condition;\n",
            "pkg/tests/error_snapshots/condition.rs": "#[test]\n#[ignore]\nfn eval() { assert_snapshot!(x); }\n",
            "pkg/tests/error_snapshots/snapshots/error_snapshots__condition__eval.snap": "eval\n",
            "pkg/tests/snapshots/level3_image__paint.snap": "paint\n",
            "pkg/src/snapshots/pkg__unit__case.snap": "unit\n",
        }
        for path, text in self.files.items():
            self.put(self.before, path, text)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def put(self, root: Path, relative: str, text: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    EXPECTED = {
        "pkg/tests/snapshots/wrap_basics__help.snap": "pkg/tests/l1/snapshots/l1__wrap_basics__help.snap",
        "pkg/tests/snapshots/wrap_basics__nested__inner.snap": "pkg/tests/l1/snapshots/l1__wrap_basics__nested__inner.snap",
        "pkg/tests/error_snapshots/snapshots/error_snapshots__condition__eval.snap": "pkg/tests/l1/error_snapshots/snapshots/l1__error_snapshots__condition__eval.snap",
        "pkg/tests/snapshots/level3_image__paint.snap": "pkg/tests/level3-terminal/snapshots/level3_terminal__level3_image__paint.snap",
    }

    def migrate(self, skip: str | None = None, alter: str | None = None) -> dict:
        for path, text in self.files.items():
            if path.endswith(".snap") and path in self.EXPECTED:
                if path == skip:
                    self.put(self.repo, path, text)
                    continue
                self.put(self.repo, self.EXPECTED[path], text + ("changed\n" if path == alter else ""))
            elif path.endswith(".snap"):
                self.put(self.repo, path, text)
        moved, unaffected, failures = tool.derive_snapshot_mapping(self.manifest, tool.SourceReader(root=self.before))
        self.assertEqual([], failures)
        return {"moved": moved, "unaffected": unaffected}

    def check(self, mapping: dict, captures: dict | None = None) -> dict:
        return tool.check_snapshots(self.manifest, tool.SourceReader(root=self.before), mapping, repo=self.repo, captures=captures)

    def test_the_rule_derives_every_new_path(self) -> None:
        mapping = self.migrate()
        self.assertEqual(self.EXPECTED, {row["old"]: row["new"] for row in mapping["moved"]})
        self.assertEqual(["pkg/src/snapshots/pkg__unit__case.snap"], [row["path"] for row in mapping["unaffected"]])

    def test_a_byte_identical_move_passes(self) -> None:
        self.assertEqual([], self.check(self.migrate())["failures"])

    def test_a_missing_mapping_row_fails(self) -> None:
        mapping = self.migrate()
        mapping["moved"] = [row for row in mapping["moved"] if "paint" not in row["old"]]
        self.assertTrue(any("level3_image__paint.snap: no mapping row" in f for f in self.check(mapping)["failures"]))

    def test_a_hand_written_row_that_disagrees_with_the_rule_fails(self) -> None:
        mapping = self.migrate()
        mapping["moved"][0] = dict(mapping["moved"][0], new="pkg/tests/l1/snapshots/wrap_basics__help.snap")
        self.assertTrue(any("disagrees with the rule" in f for f in self.check(mapping)["failures"]))

    def test_changed_bytes_a_left_behind_file_and_pending_snapshots_fail(self) -> None:
        mapping = self.migrate(alter="pkg/tests/snapshots/wrap_basics__help.snap")
        self.assertTrue(any("content changed during the move" in f for f in self.check(mapping)["failures"]))
        self.put(self.repo, "pkg/tests/snapshots/wrap_basics__help.snap", "leftover")
        self.put(self.repo, "pkg/tests/l1/snapshots/l1__wrap_basics__help.snap.new", "pending")
        failures = self.check(mapping)["failures"]
        self.assertTrue(any("old snapshot still exists" in f for f in failures))
        self.assertTrue(any("pending snapshot file exists" in f for f in failures))

    def test_an_unmoved_snapshot_must_be_unchanged(self) -> None:
        mapping = self.migrate()
        self.put(self.repo, "pkg/src/snapshots/pkg__unit__case.snap", "drift\n")
        self.assertTrue(any("unmoved snapshot changed" in f for f in self.check(mapping)["failures"]))

    def test_insta_path_settings_invalidate_the_rule(self) -> None:
        mapping = self.migrate()
        self.put(self.repo, "pkg/tests/l1/wrap_basics.rs", "insta::with_settings!({prepend_module_to_snapshot => false}, {});\n")
        self.assertTrue(any("prepend_module_to_snapshot" in f for f in self.check(mapping)["failures"]))

    def test_an_unattributable_snapshot_fails(self) -> None:
        self.put(self.before, "pkg/tests/snapshots/removed_target__x.snap", "orphan\n")
        _, _, failures = tool.derive_snapshot_mapping(self.manifest, tool.SourceReader(root=self.before))
        self.assertTrue(any("names no old target" in f for f in failures), failures)

    def test_snapshots_read_only_by_ignored_tests_are_stated_per_file(self) -> None:
        capture = make_capture("pkg", {"wrap_basics": ["help"], "error_snapshots": ["condition::eval"]}, ignored={"condition::eval"})
        readers = self.check(self.migrate(), captures_of(capture))["readers"]
        # `wrap_basics__nested__inner` has no assertion that could write it: an orphan.
        self.assertEqual({"running": 1, "unresolved": 1}, readers["wrap_basics"]["snapshots"])
        self.assertEqual(["pkg/tests/snapshots/wrap_basics__nested__inner.snap"], readers["wrap_basics"]["reader_unresolved"])
        self.assertEqual(["pkg/tests/error_snapshots/snapshots/error_snapshots__condition__eval.snap"],
                         readers["error_snapshots"]["read_by_no_running_test"])
        self.assertIn("1 of 1 moved snapshots are read only by #[ignore]d tests", readers["error_snapshots"]["statement"])
        self.assertEqual(["pkg/tests/snapshots/level3_image__paint.snap"], readers["level3_image"]["reader_unresolved"])

    def test_runtime_names_attribute_to_the_most_specific_assertion(self) -> None:
        # The biscuit-terminal layout_matrix shape: an ignored loop writes `<case>__<scenario>`,
        # running loops write `<case>__<scenario>__browser`; a nested helper and a fixture
        # `fn main` inside a raw string must not be taken for the asserting test.
        self.files = {
            "pkg/tests/wrap_basics.rs": (
                '#[test]\n#[ignore]\nfn matrix() { for c in cases() { insta::assert_snapshot!(format!("{}__{}", c.a, c.b), x); } }\n'
                '#[test]\nfn matrix_browser() { for c in cases() { insta::assert_snapshot!(format!("{}__{}__browser", c.a, c.b), x); } }\n'
                '#[test]\nfn code_block() {\n    let fixture = r#"fn main() {}"#;\n    fn helper() {}\n    insta::assert_snapshot!("code_block", fixture);\n}\n'
            ),
            "pkg/tests/snapshots/wrap_basics__Table__narrow.snap": "t\n",
            "pkg/tests/snapshots/wrap_basics__Table__narrow__browser.snap": "b\n",
            "pkg/tests/snapshots/wrap_basics__code_block.snap": "c\n",
        }
        shutil.rmtree(self.before)
        for path, text in self.files.items():
            self.put(self.before, path, text)
        self.EXPECTED = {p: p.replace("tests/snapshots/", "tests/l1/snapshots/l1__") for p in self.files if p.endswith(".snap")}
        capture = make_capture("pkg", {"wrap_basics": ["matrix", "matrix_browser", "code_block"]}, ignored={"matrix"})
        readers = self.check(self.migrate(), captures_of(capture))["readers"]["wrap_basics"]
        self.assertEqual({"ignored": 1, "running": 2}, readers["snapshots"])
        self.assertEqual(["pkg/tests/snapshots/wrap_basics__Table__narrow.snap"], readers["read_by_no_running_test"])
        self.assertEqual({"matrix": "ignored", "matrix_browser": "running", "code_block": "running"}, readers["asserting_functions"])


@unittest.skipUnless(JUST and baseline_dir(), "needs just and the Phase 1 baseline of 2026-09-21-consolidated-test-binaries")
class ShippedArtifactPlanTests(unittest.TestCase):
    """`plan` over the real Phase 1 inventory and before-listings.

    The inventory and listings are frozen evidence, so these results do not
    drift when packages migrate. Tier filters come from the live
    `_tier_filter`. Overrides are read live only for darkmatter-cli, which
    owns none of the exact-name overrides the pilot phases rewrite.
    """

    @classmethod
    def setUpClass(cls) -> None:
        base = baseline_dir()
        cls.inventory = json.loads((base / "inventory.json").read_text())
        cls.captures = tool.load_captures([base / "listings"])

    def plan(self, package: str, with_overrides: bool) -> tool.PlanResult:
        filters = {tier: tool.tier_filter(tier, package) for tier in tool.TIER_SELECTORS}
        if with_overrides:
            filters.update(tool.override_filters())
        return tool.plan_package(package, self.inventory, self.captures, filters)

    def test_darkmatter_cli_needs_exactly_the_one_known_alias(self) -> None:
        result = self.plan("darkmatter-cli", with_overrides=True)
        self.assertEqual([], result.failures)
        aliases = {m["old_target"]: m["module"] for m in result.manifest["modules"] if m["alias"]}
        self.assertEqual({"level2_harness_integrity": "harness_integrity"}, aliases)
        self.assertEqual(["l1", "level2"], [t["name"] for t in result.manifest["targets"]])

    def test_the_ruled_target_sets_for_every_package(self) -> None:
        expected = {
            "claudine-cli": {"l1": [], "level2": ["terminal-tests"], "level3": ["terminal-tests"], "real": ["real-tests"]},
            "darkmatter": {"l1": [], "level2": ["terminal-tests"], "level3-terminal": ["terminal-tests"],
                           "level3-browser": ["browser-tests"], "browser": ["browser-tests"]},
            "biscuit-terminal": {"l1": [], "level2": ["terminal-tests"]},
        }
        for package, targets in expected.items():
            with self.subTest(package=package):
                result = self.plan(package, with_overrides=False)
                self.assertEqual([], result.failures)
                self.assertEqual(targets, {t["name"]: t["required_features"] for t in result.manifest["targets"]})
                self.assertEqual([], [m["old_target"] for m in result.manifest["modules"] if m["alias"]])
                moved = sum(len(t["modules"]) for t in result.manifest["targets"])
                self.assertEqual(len(self.inventory["packages"][package]["test_targets"]), moved, "every old target maps to exactly one module")


if __name__ == "__main__":
    unittest.main(verbosity=2)
