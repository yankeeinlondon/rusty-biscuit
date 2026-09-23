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
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

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


class SharedAndRuledPlanTests(unittest.TestCase):
    def plan(self, targets: list[dict], captures: list[dict], ruled: dict | None = None) -> tool.PlanResult:
        with mock.patch.dict(tool.RULED_TARGETS, {"pkg": ruled} if ruled else {}, clear=True):
            return tool.plan_package("pkg", inventory_of(targets), captures_of(*captures), FIXTURE_FILTERS, repo=Path(tempfile.gettempdir()))

    def test_shared_common_tests_are_recorded_once_and_force_no_alias(self) -> None:
        # The biscuit-terminal-cli shape: every level2_ file compiles a copy of common's unit tests.
        suites = {"level2_a": ["level2_x", "common::geometry::tests::y"], "level2_b": ["level2_z", "common::geometry::tests::y"]}
        targets = [{"name": name, "required_features": ["t"], "declares_mod_common": True} for name in suites]
        result = self.plan(targets, [make_capture("pkg", suites, features=("t",))])
        self.assertEqual([], result.failures)
        self.assertEqual([], [m["old_target"] for m in result.manifest["modules"] if m["alias"]])
        self.assertEqual([{"target": "level2", "test": "common::geometry::tests::y", "old_binary_ids": ["pkg::level2_a", "pkg::level2_b"]}],
                         result.manifest["shared_tests"])
        self.assertEqual([{"old": "level2_x", "new": "level2_a::level2_x"}], result.manifest["modules"][0]["tests"])
        # Without `mod common;` the same path is the module's own and projects under it, forcing an alias.
        own = self.plan([dict(t, declares_mod_common=False) for t in targets], [make_capture("pkg", suites, features=("t",))])
        self.assertEqual(["level2_a", "level2_b"], [m["old_target"] for m in own.manifest["modules"] if m["alias"]])

    def test_a_ruled_l1_target_joins_the_named_target(self) -> None:
        targets = [{"name": "alpha"}, {"name": "level2_x", "required_features": ["t"]}, {"name": "win_only", "required_features": ["t"]}]
        captures = [make_capture("pkg", {"alpha": ["plain"]}), make_capture("pkg", {"alpha": ["plain"], "level2_x": ["level2_y"], "win_only": ["w"]}, features=("t",))]
        default = self.plan(targets, captures)
        self.assertEqual({"l1", "l1-t", "level2"}, {t["name"] for t in default.manifest["targets"]})
        ruled = self.plan(targets, captures, {"win_only": ("level2", "R4")})
        self.assertEqual([], ruled.failures)
        self.assertEqual({"l1": [], "level2": ["t"]}, {t["name"]: t["required_features"] for t in ruled.manifest["targets"]})
        self.assertEqual(["level2_x", "win_only"], [t for t in ruled.manifest["targets"] if t["name"] == "level2"][0]["modules"])
        self.assertEqual([{"old_target": "win_only", "target": "level2", "ruling": "R4"}], ruled.manifest["ruled_targets"])

    def test_joining_a_target_with_more_features_needs_no_test_without_them(self) -> None:
        # The sniff-cli shape (R14): no manifest feature, but an inner cfg empties the binary without it.
        targets = [{"name": "level2_gated"}, {"name": "level2_x", "required_features": ["t"]}]
        empty = [make_capture("pkg", {"level2_gated": []}), make_capture("pkg", {"level2_gated": ["level2_g"], "level2_x": ["level2_y"]}, features=("t",))]
        result = self.plan(targets, empty, {"level2_gated": ("level2", "R14")})
        self.assertEqual([], result.failures)
        self.assertEqual({"level2": ["t"]}, {t["name"]: t["required_features"] for t in result.manifest["targets"]})
        listed = [make_capture("pkg", {"level2_gated": ["level2_g"]}), empty[1]]
        failures = self.plan(targets, listed, {"level2_gated": ("level2", "R14")}).failures
        self.assertTrue(any("lists tests without ['t'] (feature sets ['none'])" in f for f in failures), failures)
        unproven = self.plan(targets, empty[1:], {"level2_gated": ("level2", "R14")}).failures
        self.assertTrue(any("no capture lacks ['t']" in f for f in unproven), unproven)

    def test_a_ruled_target_may_not_gain_compilation_where_it_had_none(self) -> None:
        targets = [{"name": "alpha"}, {"name": "gated", "required_features": ["t"]}]
        captures = [make_capture("pkg", {"alpha": ["plain"], "gated": ["g"]}, features=("t",))]
        failures = self.plan(targets, captures, {"gated": ("l1", "bad")}).failures
        self.assertTrue(any("requires ['t'], which target 'l1' does not" in f for f in failures), failures)

    def test_a_ruled_helper_must_have_no_tests_and_every_ruling_must_name_a_target(self) -> None:
        targets = [{"name": "integration"}, {"name": "fixtures"}]
        result = self.plan(targets, [make_capture("pkg", {"integration": ["t"], "fixtures": []})], {"fixtures": (None, "R19")})
        self.assertEqual([], result.failures)
        self.assertEqual(["integration"], [m["old_target"] for m in result.manifest["modules"]])
        self.assertEqual([{"old_target": "fixtures", "old_path": "pkg/tests/fixtures.rs", "ruling": "R19"}], result.manifest["dropped_targets"])
        failures = self.plan(targets, [make_capture("pkg", {"integration": ["t"], "fixtures": ["f"]})], {"fixtures": (None, "R19")}).failures
        self.assertTrue(any("ruled a helper, not a module (R19), but lists tests ['f']" in f for f in failures), failures)
        failures = self.plan(targets, [make_capture("pkg", {"integration": ["t"], "fixtures": []})], {"ghost": ("l1", "x")}).failures
        self.assertTrue(any("RULED_TARGETS names 'ghost'" in f for f in failures), failures)

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

    SHARED_BEFORE = {"level2_a": ["level2_x", "common::y"], "level2_b": ["level2_z", "common::y"]}
    SHARED_AFTER = {"level2": ["level2_a::level2_x", "level2_b::level2_z", "common::y"]}

    def shared_manifest(self, shared: bool = True) -> dict:
        manifest = manifest_for("pkg", [("level2_a", "level2", "level2_a"), ("level2_b", "level2", "level2_b")])
        manifest["shared_tests"] = [{"target": "level2", "test": "common::y", "old_binary_ids": ["pkg::level2_a", "pkg::level2_b"]}] if shared else []
        return manifest

    def test_shared_common_copies_fold_into_the_consolidated_identity(self) -> None:
        report = self.compare([make_capture("pkg", self.SHARED_BEFORE)], [make_capture("pkg", self.SHARED_AFTER)], manifest=self.shared_manifest())
        self.assertEqual([], report["failures"])
        present = report["packages"]["pkg"]["cells"][0]["selectors"]["L1"]["sets"]["present"]
        self.assertEqual((3, 3), (present["before"], present["after"]), "two copies are one identity")
        unshared = self.compare([make_capture("pkg", self.SHARED_BEFORE)], [make_capture("pkg", self.SHARED_AFTER)], manifest=self.shared_manifest(False))
        self.assertTrue(any("common::y is not mapped" in f for f in unshared["failures"]), unshared["failures"])

    def test_shared_copies_that_disagree_fail(self) -> None:
        before = make_capture("pkg", self.SHARED_BEFORE)
        before["suites"]["pkg::level2_b"]["tests"]["common::y"]["ignored"] = True
        report = self.compare([before], [make_capture("pkg", self.SHARED_AFTER)], manifest=self.shared_manifest())
        self.assertTrue(any("copies of shared test common::y disagree" in f for f in report["failures"]), report["failures"])

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


class PackageTableTests(unittest.TestCase):
    def test_the_wave_2_feature_sets_are_the_ruled_table(self) -> None:
        # 2026-09-22-consolidated-test-binaries-wave-2 rulings R1 (from spikes/s1-feature-sets.md).
        self.assertEqual({
            "tree-hugger": ((),),
            "claudine": ((),),
            "sniff": ((), ("remote",), ("network",)),
            "sniff-cli": ((), ("test-fixtures",)),
            "biscuit-file": ((), ("fetch",)),
            "schematic-gen": ((), ("terminal-tests",)),
            "biscuit-terminal-cli": ((), ("terminal-tests",)),
            "claudine-gen": ((), ("terminal-tests",)),
            "dmls": ((), ("effects-instrumentation",), ("terminal-tests",), ("terminal-tests", "effects-instrumentation")),
            "biscuit-tui-cli": ((), ("terminal-tests",)),
        }, {package: tool.PACKAGE_FEATURE_SETS[package] for package in tool.WAVE_2_PACKAGES})
        self.assertEqual(set(tool.PACKAGES), set(tool.PACKAGE_FEATURE_SETS))

    def test_an_unlisted_package_is_refused_before_any_subprocess(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(tool.ToolError, "no feature sets are known for worktree-cli"):
                tool.capture_package("worktree-cli", Path(temporary), repo=Path(temporary))
            self.assertEqual([], list(Path(temporary).iterdir()), "nothing was written")
        with contextlib.redirect_stderr(io.StringIO()) as stderr:
            self.assertEqual(2, tool.main(["inventory", "--package", "worktree-cli"]))
        self.assertIn("worktree-cli not in PACKAGES", stderr.getvalue())


@unittest.skipUnless(shutil.which("cargo"), "cargo is not installed")
class ShippedPackageTableTests(unittest.TestCase):
    """Every wave-2 feature set against the package's real `Cargo.toml`.

    Wave-1 rows are that feature's frozen record and are not re-validated
    here: `claudine-cli`'s CI union gained `test-fixtures` after it closed.
    """

    def test_every_listed_set_names_declared_features_and_holds_the_ci_union(self) -> None:
        metadata = tool.cargo_metadata()
        manifests = {p["name"]: Path(p["manifest_path"]) for p in metadata["packages"]}
        for package in tool.WAVE_2_PACKAGES:
            sets = tool.PACKAGE_FEATURE_SETS[package]
            with self.subTest(package=package):
                self.assertIn(package, manifests)
                declared = set(tool.tomllib.loads(manifests[package].read_text()).get("features", {}))
                for features in sets:
                    self.assertLessEqual(set(features), declared, f"{features} names an undeclared feature")
                ci_union = set(tool.package_ci_tests(manifests[package]).get("features", []))
                self.assertIn(ci_union, [set(s) for s in sets], "capture refuses a package whose CI union is not listed")


class ProptestPathTests(unittest.TestCase):
    def test_resolution_follows_the_nearest_crate_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            tests = Path(temporary) / "pkg" / "tests"
            (tests / "l1" / "nested").mkdir(parents=True)
            self.assertEqual(tests / "yaml.proptest-regressions", tool.proptest_regression_path(tests / "yaml.rs"),
                             "no lib.rs/main.rs above a top-level test file: the sibling fallback")
            (tests / "l1" / "main.rs").write_text("")
            self.assertEqual(tests / "proptest-regressions" / "yaml.txt", tool.proptest_regression_path(tests / "l1" / "yaml.rs"))
            self.assertEqual(tests / "proptest-regressions" / "nested" / "mod.txt", tool.proptest_regression_path(tests / "l1" / "nested" / "mod.rs"))


class MoveFixture(unittest.TestCase):
    """A package with every construct `move` repairs, before and after."""

    FILES = {
        "pkg/tests/alpha.rs": '//! Unix only.\n#![cfg(unix)]\n\nmod common;\nconst Q: &str = include_str!("fixtures/q.txt");\n#[test]\nfn plain() {}\n',
        "pkg/tests/level2_prose.rs": ('#[path = "common/mod.rs"]\nmod common;\n#[path = "../benches/support/b.rs"]\nmod b;\n'
                                      'const Q: &[u8] = include_bytes!("/abs/q.txt");\n#[test]\nfn plain() {}\n'),
        "pkg/tests/beta.rs": "mod helper;\n#[test]\nfn plain() {}\n",
        "pkg/tests/gamma.rs": "#[cfg(unix)]\nmod common;\n#[test]\nfn plain() {}\n",
        "pkg/tests/nested/main.rs": "mod child;\n",
        "pkg/tests/nested/child.rs": 'const Q: &str = include_str!("../fixtures/q.txt");\n#[test]\nfn n() {}\n',
        "pkg/tests/yaml.rs": "proptest::proptest! {}\n",
        "pkg/tests/yaml.proptest-regressions": "cc 0123abcd # shrinks to x = 0\n",
        "pkg/tests/windows_only.rs": "//! Windows.\n#![cfg(windows)]\n\n#[test]\nfn w() {}\n",
        "pkg/tests/solo.rs": "#[test]\nfn level3_s() {}\n",
        "pkg/tests/common/mod.rs": "pub fn shared() {}\n",
        "pkg/tests/helper.rs": "pub fn help() {}\n",
        "pkg/tests/fixtures/q.txt": "q\n",
        "pkg/benches/support/b.rs": "",
    }

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="consolidation-move-")
        root = Path(self.temporary.name)
        self.repo, self.before = root / "repo", root / "before"
        for path, text in self.FILES.items():
            for base in (self.repo, self.before):
                (base / path).parent.mkdir(parents=True, exist_ok=True)
                (base / path).write_text(text)
        row = lambda old, target, module, **extra: {
            "old_target": old, "old_binary_id": f"pkg::{old}", "old_path": f"pkg/tests/{old}.rs", "nested_root": False,
            "target": target, "binary_id": f"pkg::{target}", "module": module, "new_path": f"pkg/tests/{target}/{module}.rs",
            "alias": None, "inner_cfg": [], **extra}
        self.manifest = {
            "package": "pkg", "manifest": "pkg/Cargo.toml", "crate_dir": "pkg", "dispositions": [], "additions": [],
            "targets": [
                {"name": "l1", "path": "pkg/tests/l1/main.rs", "tier": "L1", "required_features": [],
                 "modules": ["yaml", "nested", "gamma", "beta", "alpha"]},
                {"name": "level2", "path": "pkg/tests/level2/main.rs", "tier": "L2", "required_features": ["t"],
                 "modules": ["windows_only", "prose"]},
                {"name": "level3", "path": "pkg/tests/level3/main.rs", "tier": "L3", "required_features": ["t"], "modules": ["solo"]},
            ],
            "modules": [
                row("alpha", "l1", "alpha", inner_cfg=["unix"]),
                row("beta", "l1", "beta"),
                row("gamma", "l1", "gamma"),
                dict(row("nested", "l1", "nested"), old_path="pkg/tests/nested/main.rs", new_path="pkg/tests/l1/nested/mod.rs", nested_root=True),
                row("yaml", "l1", "yaml"),
                row("level2_prose", "level2", "prose", alias={"from": "level2_prose", "reason": ["L1: plain matches before"]}),
                row("windows_only", "level2", "windows_only", inner_cfg=["windows"], keep_inner_cfg=True),
                row("solo", "level3", "solo"),
            ],
        }

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def text(self, relative: str) -> str:
        return (self.repo / relative).read_text()

    def move(self) -> dict:
        return tool.move_package(self.manifest, repo=self.repo)


class MoveTests(MoveFixture):
    def test_files_move_and_relative_constructs_keep_their_targets(self) -> None:
        summary = self.move()
        for row in self.manifest["modules"]:
            self.assertFalse((self.repo / row["old_path"]).exists(), row["old_path"])
            self.assertTrue((self.repo / row["new_path"]).is_file(), row["new_path"])
        self.assertEqual('//! Unix only.\nuse crate::common;\nconst Q: &str = include_str!("../fixtures/q.txt");\n#[test]\nfn plain() {}\n',
                         self.text("pkg/tests/l1/alpha.rs"))
        self.assertEqual('#[path = "../helper.rs"]\nmod helper;\n#[test]\nfn plain() {}\n', self.text("pkg/tests/l1/beta.rs"))
        self.assertEqual("#[cfg(unix)]\nuse crate::common;\n#[test]\nfn plain() {}\n", self.text("pkg/tests/l1/gamma.rs"))
        self.assertEqual("mod child;\n", self.text("pkg/tests/l1/nested/mod.rs"), "a nested root's own children move with it")
        self.assertEqual('const Q: &str = include_str!("../../fixtures/q.txt");\n#[test]\nfn n() {}\n', self.text("pkg/tests/l1/nested/child.rs"))
        self.assertEqual({"old": "pkg/tests/alpha.rs", "new": "pkg/tests/l1/alpha.rs"}, summary["moved"][0])
        self.assertEqual(["fixtures/q.txt → ../fixtures/q.txt", "mod common; → use crate::common;"],
                         sorted(summary["path_repairs"]["pkg/tests/l1/alpha.rs"]))

    def test_an_alias_from_the_manifest_becomes_the_module_name(self) -> None:
        summary = self.move()
        self.assertFalse((self.repo / "pkg/tests/level2/level2_prose.rs").exists())
        self.assertEqual('mod common;\n#[path = "../../benches/support/b.rs"]\nmod b;\nconst Q: &[u8] = include_bytes!("/abs/q.txt");\n#[test]\nfn plain() {}\n'
                         .replace("mod common;", "use crate::common;"), self.text("pkg/tests/level2/prose.rs"))
        root = self.text("pkg/tests/level2/main.rs")
        self.assertIn("\nmod prose;\n", root)
        self.assertNotIn("level2_prose", root)
        self.assertEqual(["../benches/support/b.rs → ../../benches/support/b.rs", "mod common; → use crate::common;"],
                         sorted(summary["path_repairs"]["pkg/tests/level2/prose.rs"]), "a dropped #[path] is not logged as repaired")

    def test_roots_declare_common_only_where_used_and_modules_in_rustfmt_order(self) -> None:
        self.move()
        l1 = self.text("pkg/tests/l1/main.rs")
        self.assertTrue(l1.startswith("//! Level 1 integration tests for `pkg`, one test binary per\n"), l1)
        self.assertIn("never compiles; `test_layout.rs` rejects one.\n", l1)
        self.assertTrue(l1.endswith('\n#[path = "../common/mod.rs"]\nmod common;\n\n#[cfg(unix)]\nmod alpha;\nmod beta;\nmod gamma;\nmod nested;\nmod yaml;\n'), l1)
        self.assertTrue(self.text("pkg/tests/level2/main.rs").endswith('mod common;\n\nmod prose;\n#[cfg(windows)]\nmod windows_only;\n'))
        level3 = self.text("pkg/tests/level3/main.rs")
        self.assertTrue(level3.startswith("//! Level 3 (`t`) integration tests"), level3)
        self.assertNotIn("mod common;", level3, "no module of level3 uses common")

    @unittest.skipUnless(shutil.which("rustfmt"), "rustfmt is not installed")
    def test_a_generated_root_is_already_rustfmt_clean(self) -> None:
        self.manifest["targets"][0]["modules"] += ["Zed", "a10", "a2", "a_b", "ab"]
        for name in ("Zed", "a10", "a2", "a_b", "ab"):
            self.manifest["modules"].append(dict(self.manifest["modules"][1], old_target=name, old_path=f"pkg/tests/{name}.rs",
                                                 module=name, new_path=f"pkg/tests/l1/{name}.rs"))
            (self.repo / f"pkg/tests/{name}.rs").write_text("")
        self.move()
        completed = subprocess.run(["rustfmt", "--check", "--edition", "2024", str(self.repo / "pkg/tests/l1/main.rs")],
                                   capture_output=True, text=True)
        # rustfmt also visits the child modules; only the root's own diff matters here.
        self.assertNotIn("l1/main.rs", completed.stdout, completed.stdout)

    def test_the_inner_cfg_is_kept_only_where_the_row_says_so(self) -> None:
        self.move()
        self.assertEqual("//! Windows.\n#![cfg(windows)]\n\n#[test]\nfn w() {}\n", self.text("pkg/tests/level2/windows_only.rs"))
        self.assertNotIn("#![cfg", self.text("pkg/tests/l1/alpha.rs"))

    def test_proptest_seeds_move_to_where_proptest_reads_them(self) -> None:
        summary = self.move()
        seed = self.repo / "pkg/tests/proptest-regressions/yaml.txt"
        self.assertEqual(self.FILES["pkg/tests/yaml.proptest-regressions"], seed.read_text())
        self.assertFalse((self.repo / "pkg/tests/yaml.proptest-regressions").exists())
        self.assertFalse((self.repo / "pkg/tests/l1/yaml.proptest-regressions").exists(), "beside the module is where wave 1 went wrong")
        self.assertEqual(seed, tool.proptest_regression_path(self.repo / "pkg/tests/l1/yaml.rs"))
        self.assertEqual([{"old": "pkg/tests/yaml.proptest-regressions", "new": "pkg/tests/proptest-regressions/yaml.txt"}],
                         summary["proptest_relocated"])

    def test_the_cargo_entries_carry_exact_required_features(self) -> None:
        entries = self.move()["cargo_test_entries"]
        self.assertIn('[[test]]\nname = "l1"\npath = "tests/l1/main.rs"\n\n', entries)
        self.assertIn('[[test]]\nname = "level2"\npath = "tests/level2/main.rs"\nrequired-features = ["t"]\n', entries)

    def test_the_moved_tree_passes_every_after_check(self) -> None:
        self.move()
        before = tool.SourceReader(root=self.before)
        attributes = tool.check_attributes(self.manifest, before, repo=self.repo)
        self.assertEqual([], attributes["failures"])
        self.assertIn({"path": "pkg/tests/level2/windows_only.rs", "kind": "inner-cfg-duplicated", "attribute": "cfg(windows)"},
                      attributes["review"])
        self.assertEqual([], tool.check_proptest(self.manifest, before, repo=self.repo)["failures"])
        self.assertEqual([], tool.body_diff(self.manifest, before, tool.SourceReader(root=self.repo))["failures"])

    def test_refusals_leave_the_tree_untouched(self) -> None:
        cases = {
            "destination already exists": lambda: (self.repo / "pkg/tests/l1").mkdir() or (self.repo / "pkg/tests/l1/beta.rs").write_text(""),
            "resolves to no file": lambda: (self.repo / "pkg/tests/helper.rs").unlink(),
            "expected one #!\\[cfg\\(unix\\)\\]": lambda: (self.repo / "pkg/tests/alpha.rs").write_text("mod common;\n"),
            "declares modules with no manifest row": lambda: self.manifest["targets"][2]["modules"].append("ghost"),
        }
        for message, breakage in cases.items():
            with self.subTest(message=message):
                self.tearDown()
                self.setUp()
                breakage()
                with self.assertRaisesRegex(tool.ToolError, message):
                    self.move()
                self.assertTrue((self.repo / "pkg/tests/gamma.rs").is_file(), "nothing moved")
                self.assertFalse((self.repo / "pkg/tests/l1/main.rs").exists(), "no root written")

    def test_cli_move_writes_a_summary(self) -> None:
        manifest = Path(self.temporary.name) / "manifest.json"
        manifest.write_text(json.dumps(self.manifest))
        original = tool.REPO_ROOT
        tool.REPO_ROOT = self.repo
        try:
            with contextlib.redirect_stderr(io.StringIO()) as stderr:
                code = tool.main(["move", str(manifest), "--out", str(Path(self.temporary.name) / "summary.json")])
        finally:
            tool.REPO_ROOT = original
        self.assertEqual(0, code, stderr.getvalue())
        self.assertEqual("consolidation-move", json.loads((Path(self.temporary.name) / "summary.json").read_text())["kind"])
        self.assertIn('name = "level3"', stderr.getvalue())


class CheckProptestTests(MoveFixture):
    def check(self) -> dict:
        return tool.check_proptest(self.manifest, tool.SourceReader(root=self.before), repo=self.repo)

    def test_a_seed_left_beside_its_module_is_never_replayed(self) -> None:
        # The wave-1 darkmatter shape: moved next to the module, where proptest no longer looks.
        self.move()
        (self.repo / "pkg/tests/proptest-regressions/yaml.txt").rename(self.repo / "pkg/tests/l1/yaml.proptest-regressions")
        failures = self.check()["failures"]
        self.assertEqual(1, len(failures), failures)
        self.assertIn("pkg/tests/l1/yaml.proptest-regressions: no test source resolves to this seed file", failures[0])

    def test_changed_or_lost_seeds_fail(self) -> None:
        self.move()
        seed = self.repo / "pkg/tests/proptest-regressions/yaml.txt"
        seed.write_text("cc regenerated\n")
        self.assertTrue(any("move byte for byte" in f for f in self.check()["failures"]))
        seed.unlink()
        self.assertTrue(any("nothing" in f for f in self.check()["failures"]))

    def test_a_correct_relocation_is_reported(self) -> None:
        self.move()
        report = self.check()
        self.assertEqual([], report["failures"])
        self.assertEqual([{"old": "pkg/tests/yaml.proptest-regressions", "new": "pkg/tests/proptest-regressions/yaml.txt"}], report["relocated"])


class BodyDiffTests(MoveFixture):
    def diff(self) -> dict:
        return tool.body_diff(self.manifest, tool.SourceReader(root=self.before), tool.SourceReader(root=self.repo))

    def test_structural_edits_pass_and_are_counted(self) -> None:
        self.move()
        report = self.diff()
        self.assertEqual([], report["failures"])
        self.assertEqual(9, report["files"], "8 module files plus the nested root's child")
        self.assertGreater(report["changed_lines"]["structural"], 0)
        self.assertEqual(0, report["changed_lines"]["other"])

    def test_a_body_change_fails_with_the_line(self) -> None:
        self.move()
        path = self.repo / "pkg/tests/l1/nested/child.rs"
        path.write_text(path.read_text().replace("fn n() {}", "fn n() { assert!(false); }"))
        report = self.diff()
        self.assertEqual(1, len(report["failures"]), report["failures"])
        self.assertIn("pkg/tests/nested/child.rs → pkg/tests/l1/nested/child.rs", report["failures"][0])
        self.assertIn("+ fn n() { assert!(false); }", report["details"])
        self.assertIn("## Failures", tool.render_body_diff_markdown(report, "base", "the working tree"))

    def test_a_missing_module_fails(self) -> None:
        self.move()
        (self.repo / "pkg/tests/l1/beta.rs").unlink()
        self.assertTrue(any("missing on one side" in f for f in self.diff()["failures"]))

    def plant_self_exec(self, after_line: str) -> Path:
        """`beta` re-execs its own binary by test path; the move prefixes the module (R19)."""
        before_line = 'let args = ["--exact", "probe"];\n'
        (self.before / "pkg/tests/beta.rs").write_text(f"mod helper;\n{before_line}#[test]\nfn plain() {{}}\n")
        self.move()
        moved = self.repo / "pkg/tests/l1/beta.rs"
        moved.write_text(moved.read_text().replace("#[test]", f"{after_line}#[test]", 1))
        return moved

    def test_a_dispositioned_identity_repair_is_structural(self) -> None:
        self.plant_self_exec('let args = ["--exact", "beta::probe"];\n')
        self.manifest["dispositions"].append({"path": "pkg/tests/l1/beta.rs", "detector": "exact_arg", "reason": "R19"})
        report = self.diff()
        self.assertEqual([], report["failures"])
        self.assertEqual(0, report["changed_lines"]["other"])

    def test_an_identity_repair_without_a_disposition_fails(self) -> None:
        self.plant_self_exec('let args = ["--exact", "beta::probe"];\n')
        report = self.diff()
        self.assertIn('+ let args = ["--exact", "beta::probe"];', report["details"])

    def test_a_dispositioned_file_still_fails_on_a_body_change(self) -> None:
        self.plant_self_exec('let args = ["--exact", "beta::other_probe"];\n')
        self.manifest["dispositions"].append({"path": "pkg/tests/l1/beta.rs", "detector": "exact_arg", "reason": "R19"})
        report = self.diff()
        self.assertIn('+ let args = ["--exact", "beta::other_probe"];', report["details"])


class CheckMetadataTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="consolidation-metadata-")
        self.repo = Path(self.temporary.name)
        (self.repo / "pkg").mkdir()
        (self.repo / "pkg/Cargo.toml").write_text('[package]\nname = "pkg"\nautotests = false\n')
        self.manifest = {
            "package": "pkg", "manifest": "pkg/Cargo.toml",
            "targets": [{"name": "l1", "path": "pkg/tests/l1/main.rs", "required_features": []},
                        {"name": "level2", "path": "pkg/tests/level2/main.rs", "required_features": ["b", "a"]}],
            "modules": [{"old_target": "alpha", "target": "l1"}, {"old_target": "level2_x", "target": "level2"}],
        }
        self.targets = [("l1", "pkg/tests/l1/main.rs", []), ("level2", "pkg/tests/level2/main.rs", ["a", "b"])]

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def check(self) -> list[str]:
        metadata = {"packages": [{"name": "pkg", "targets": [
            {"name": name, "kind": ["test"], "src_path": str(self.repo / path), "required-features": features}
            for name, path, features in self.targets] + [{"name": "pkg", "kind": ["lib"], "src_path": str(self.repo / "pkg/src/lib.rs")}]}]}
        return tool.check_metadata([self.manifest], metadata, repo=self.repo)[1]

    def test_matching_targets_pass(self) -> None:
        self.assertEqual([], self.check())

    def test_each_mismatch_is_named(self) -> None:
        cases = {
            "declared target absent from Cargo ('level2'": lambda: self.targets.pop(),
            "undeclared Cargo test target ('stray'": lambda: self.targets.append(("stray", "pkg/tests/stray.rs", [])),
            "undeclared Cargo test target ('level2', 'pkg/tests/level2/main.rs', ('a',))": lambda: self.targets.__setitem__(1, ("level2", "pkg/tests/level2/main.rs", ["a"])),
            "`autotests = false` missing": lambda: (self.repo / "pkg/Cargo.toml").write_text('[package]\nname = "pkg"\n'),
            "old target alpha maps to 2 modules": lambda: self.manifest["modules"].append({"old_target": "alpha", "target": "l1"}),
            "maps to unknown target level9": lambda: self.manifest["modules"].append({"old_target": "gone", "target": "level9"}),
        }
        for message, breakage in cases.items():
            with self.subTest(message=message):
                self.tearDown()
                self.setUp()
                breakage()
                failures = self.check()
                self.assertTrue(any(message in f for f in failures), failures)

    def test_cli_exit_codes(self) -> None:
        manifest = self.repo / "m.json"
        manifest.write_text(json.dumps(self.manifest))
        metadata = self.repo / "metadata.json"
        original = tool.REPO_ROOT
        tool.REPO_ROOT = self.repo
        try:
            for targets, expected in ((self.targets, 0), (self.targets[:1], 1)):
                metadata.write_text(json.dumps({"packages": [{"name": "pkg", "targets": [
                    {"name": n, "kind": ["test"], "src_path": str(self.repo / p), "required-features": f} for n, p, f in targets]}]}))
                with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(expected, tool.main(["check-metadata", "--manifest", str(manifest), "--metadata", str(metadata)]))
        finally:
            tool.REPO_ROOT = original


def wave_1_manifests() -> list[Path]:
    return sorted((REPO / "features").glob("**/2026-09-21-consolidated-test-binaries/*-migration.json"))


@unittest.skipUnless(shutil.which("cargo") and wave_1_manifests(), "needs cargo and the wave-1 migration manifests")
class ShippedWave1MetadataTests(unittest.TestCase):
    """The ported `check-metadata` holds on the four packages wave 1 migrated."""

    def test_the_wave_1_manifests_still_match_cargo(self) -> None:
        manifests = [json.loads(path.read_text()) for path in wave_1_manifests()]
        self.assertEqual(["biscuit-terminal", "claudine-cli", "darkmatter", "darkmatter-cli"], sorted(m["package"] for m in manifests))
        summary, failures = tool.check_metadata(manifests, tool.cargo_metadata())
        self.assertEqual([], failures)
        self.assertEqual(4, len(summary))


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



def wave_2_selfproof() -> Path | None:
    found = sorted((REPO / "features").glob("**/2026-09-22-consolidated-test-binaries-wave-2/selfproof"))
    return found[0] if found and (found[0] / "inventory.json").is_file() and (found[0] / "capture-a").is_dir() else None


@unittest.skipUnless(wave_2_selfproof(), "needs the Phase 2 self-proof of 2026-09-22-consolidated-test-binaries-wave-2")
class ShippedWave2PlanTests(unittest.TestCase):
    """`plan` over the frozen wave-2 inventory and capture, with the filters each capture recorded.

    Nothing here reads the live tree, so the result does not drift as the
    packages migrate. The expected shapes are the plan's target table and
    rulings R3, R4, R5, R10, R14, and R19.
    """

    EXPECTED = {
        "tree-hugger": ({"l1": []}, {}),
        "claudine": ({"l1": []}, {}),
        "sniff": ({"l1": []}, {}),
        "biscuit-file": ({"l1": [], "l1-fetch": ["fetch"]}, {}),
        "schematic-gen": ({"l1": [], "level2": ["terminal-tests"]}, {}),
        "biscuit-terminal-cli": ({"l1": [], "level2": ["terminal-tests"]}, {"level2_prose_cells": "prose_cells", "level2_diagrams": "diagrams"}),
        "claudine-gen": ({"l1": [], "level2": ["terminal-tests"]}, {}),
        "dmls": ({"l1": [], "level2": ["terminal-tests"]}, {}),
        "sniff-cli": ({"l1": [], "level2": ["test-fixtures"]}, {}),
        "biscuit-tui-cli": ({"l1": [], "level2": ["terminal-tests"], "level3": ["terminal-tests"]}, {"real_terminal_render": "terminal_render"}),
    }

    @classmethod
    def setUpClass(cls) -> None:
        base = wave_2_selfproof()
        cls.inventory = json.loads((base / "inventory.json").read_text())
        cls.captures = tool.load_captures([base / "capture-a"])
        cls.results = {}
        for package in cls.EXPECTED:
            capture = next(c for (_h, p, _fs), c in sorted(cls.captures.items()) if p == package)
            filters = {selector: meta["filter"] for selector, meta in capture["selectors"].items()}
            cls.results[package] = tool.plan_package(package, cls.inventory, cls.captures, filters)

    def test_every_package_plans_to_the_ruled_targets_and_aliases(self) -> None:
        for package, (targets, aliases) in self.EXPECTED.items():
            with self.subTest(package=package):
                result = self.results[package]
                self.assertEqual([], result.failures)
                self.assertEqual(targets, {t["name"]: t["required_features"] for t in result.manifest["targets"]})
                self.assertEqual(aliases, {m["old_target"]: m["module"] for m in result.manifest["modules"] if m["alias"]})
                mapped = len(result.manifest["modules"]) + len(result.manifest["dropped_targets"])
                self.assertEqual(len(self.inventory["packages"][package]["test_targets"]), mapped, "every old target is a module or a ruled helper")
        self.assertEqual(18, sum(len(self.EXPECTED[p][0]) for p in self.EXPECTED), "the plan's expected total")

    def test_the_two_exact_overrides_are_rewrites(self) -> None:
        suggested = {package: sorted({r["suggested_filter"] for r in result.manifest["override_rewrites"]})
                     for package, result in self.results.items() if result.manifest["override_rewrites"]}
        self.assertEqual({
            "sniff": ["test(=integration::test_detect_completes_in_reasonable_time)"],
            "biscuit-terminal-cli": ["test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)"],
        }, suggested)

    def test_rulings_and_shared_tests_are_recorded(self) -> None:
        self.assertEqual(["fixtures"], [d["old_target"] for d in self.results["sniff"].manifest["dropped_targets"]])
        self.assertEqual({"real_terminal_render": "level2", "windows_captured_stdout": "level2"},
                         {r["old_target"]: r["target"] for r in self.results["biscuit-tui-cli"].manifest["ruled_targets"]})
        shared = self.results["biscuit-terminal-cli"].manifest["shared_tests"]
        self.assertEqual(10, len(shared))
        self.assertTrue(all(len(entry["old_binary_ids"]) == 9 and entry["test"].startswith("common::pane_geometry::tests::") for entry in shared))
        self.assertEqual([], [p for p, r in self.results.items() if p != "biscuit-terminal-cli" and r.manifest["shared_tests"]])

if __name__ == "__main__":
    unittest.main(verbosity=2)
