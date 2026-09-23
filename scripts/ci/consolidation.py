#!/usr/bin/env python3
"""Migration toolkit and oracles for consolidating integration-test binaries.

Each migrated package replaces one test executable per `tests/*.rs` file with
one binary per execution contract (tier × exact `required-features` × harness ×
target-wide settings). This tool plans that move and proves afterward that no
test changed identity, tier, or reachability except by the one intentional
change: `<old-binary>::<path>` becomes `<consolidated-binary>::<module>::<path>`.

The design authority is `2026-09-21-consolidated-test-binaries` (spec, plan
Phase 2, `rulings.md`, `spikes/`). Python 3 stdlib only (ruling R1). It runs
locally and on the standing build hosts; no CI workflow invokes it.

## Examples

```bash
python3 scripts/ci/consolidation.py inventory --listings <captures> --out inventory.json
python3 scripts/ci/consolidation.py capture --package claudine-cli --out <captures>
python3 scripts/ci/consolidation.py plan --package claudine-cli --listings <captures> --out manifest.json
python3 scripts/ci/consolidation.py compare --before <captures> --after <captures> --manifest manifest.json
python3 scripts/ci/consolidation.py check-attributes --manifest manifest.json --before-rev HEAD
python3 scripts/ci/consolidation.py check-snapshots --manifest manifest.json --mapping snapshots.json
```

## Exit status

`0` when every check holds, `1` when a check failed (each failure is named on
stderr), `2` when an input could not be read or a subprocess failed. A tool
error is never reported as a test difference.

## Notes

- Tier expressions come only from `just _tier_filter <tier> <package>`, with
  `BISCUIT_TEST_FILTER` removed and `BISCUIT_L1_INCLUDE_SLOW` set explicitly.
  Override filters are read verbatim from `.config/nextest.toml`. Nothing here
  spells a tier expression itself.
- `plan` evaluates those strings with a small filterset evaluator to project
  moved paths before any file moves. It first checks the evaluator against
  every verdict Nextest itself recorded in the supplied captures and refuses
  to plan if one disagrees. `compare` never evaluates a filter: it reads
  Nextest's own verdicts from before and after captures.
- Identities are compared as exact sets, never as counts.
"""

from __future__ import annotations

import argparse
import fnmatch
import gzip
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
import tomllib
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Iterable

REPO_ROOT = Path(__file__).resolve().parents[2]

PACKAGES = ("claudine-cli", "darkmatter", "darkmatter-cli", "biscuit-terminal")

#: Every `--features` value a package's canonical recipes pass, plus the CI
#: union from `[package.metadata.ci.tests].features` (validated at capture
#: time). The order inside each set is the file-name order the Phase 1
#: baseline used.
PACKAGE_FEATURE_SETS: dict[str, tuple[tuple[str, ...], ...]] = {
    "claudine-cli": ((), ("daemon-tests",), ("terminal-tests",), ("daemon-tests", "terminal-tests"), ("real-tests",)),
    "darkmatter": (
        (),
        ("effects-instrumentation",),
        ("terminal-tests",),
        ("browser-tests",),
        ("terminal-tests", "browser-tests"),
        ("terminal-tests", "browser-tests", "effects-instrumentation"),
    ),
    "darkmatter-cli": ((), ("terminal-tests",)),
    "biscuit-terminal": ((), ("image",), ("terminal-tests",), ("browser-tests",), ("image", "terminal-tests", "browser-tests")),
}

#: Selectors produced by `just _tier_filter`, in capture order.
TIER_SELECTORS = ("L1", "sanity", "L2", "L3", "browser", "real")
#: Extra selector for packages whose manifest sets `l1-include-slow = true`.
INCLUDE_SLOW_SELECTOR = "L1-include-slow"

#: Tier markers as `_tier_filter` anchors them. Used to name targets and
#: derive aliases, never to select tests.
MARKER_TIERS = (("level2_", "L2"), ("level3_", "L3"), ("browser_", "browser"), ("real_", "real"), ("slow_", "L1"))
#: `_tier_filter` adds `perf_` to the L1 exclusion for worktree-cli only.
PACKAGE_MARKERS = {"worktree-cli": ("perf_",)}
TIER_TARGET_NAMES = {"L1": "l1", "L2": "level2", "L3": "level3", "browser": "browser", "real": "real"}
#: Keys Cargo honors on a `[[test]]` entry. Any key other than name/path is a
#: target-wide setting (R3).
TARGET_WIDE_KEYS = ("harness", "required-features", "edition", "test", "doctest", "bench", "doc", "proc-macro", "crate-type")
#: Module names a consolidated root owns itself (S1 §5).
RESERVED_MODULES = ("common", "main")
#: A module name no filter can react to, used to tell a name hazard apart from
#: a change every module name would cause (an exact-name override).
NEUTRAL_PROBE_MODULE = "consolidation_neutral_probe"

#: Fields of a raw listing that describe where it was produced, not what it
#: contains (S1). `package-id` embeds the checkout path.
LOCALITY_SUITE_FIELDS = ("binary-path", "cwd", "package-id")

#: Attributes rustc honors only at a crate root; inside a module they degrade
#: to a warning and the setting is dropped (S1 §7).
CRATE_ROOT_ONLY = (
    "recursion_limit", "type_length_limit", "no_std", "no_main", "no_implicit_prelude",
    "feature", "windows_subsystem", "crate_type", "crate_name", "test_runner",
    "reexport_test_harness_main", "no_builtins", "compiler_builtins", "moved_panic_handler",
)
LINT_ATTRIBUTES = ("allow", "warn", "deny", "expect", "forbid")

#: Crate-global constructs (S2 method step 3). Any hit fails unless the
#: manifest records a disposition for it.
CRATE_GLOBAL_DETECTORS: dict[str, re.Pattern[str]] = {
    "macro_export": re.compile(r"#\[\s*macro_export"),
    "global_allocator": re.compile(r"#\[\s*global_allocator"),
    "no_mangle": re.compile(r"#\[\s*(unsafe\s*\(\s*)?no_mangle"),
    "export_name": re.compile(r"#\[\s*(unsafe\s*\(\s*)?export_name"),
    "link_section": re.compile(r"#\[\s*(unsafe\s*\(\s*)?link_section"),
    "used": re.compile(r"#\[\s*used\b"),
    "ctor": re.compile(r"#\[\s*(ctor|dtor|ctor::ctor|dtor::dtor)\b"),
    "panic_handler": re.compile(r"#\[\s*panic_handler"),
    "extern_c_fn": re.compile(r"(pub\s+)?extern\s+\"C\"\s+fn\s+\w+"),
}
#: Constructs whose runtime behavior depends on the test path or the binary
#: (S2, R9). Each hit needs a manifest disposition.
IDENTITY_DETECTORS: dict[str, re.Pattern[str]] = {
    "exact_path_string": re.compile(r"--exact\s+[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)*"),
    "exact_arg": re.compile(r"\"--exact\""),
    "file_macro": re.compile(r"\bfile!\s*\(\s*\)"),
    "module_path_macro": re.compile(r"\bmodule_path!\s*\(\s*\)"),
    "crate_name_env": re.compile(r"CARGO_CRATE_NAME"),
}
CURRENT_EXE = re.compile(r"current_exe\s*\(")
LIST_ARG = re.compile(r"--list\b")
#: Constructs that resolve relative to the containing file (S2). Reported with
#: old and new directory for review; the compiler rejects a missed repair.
PATH_DETECTORS: dict[str, re.Pattern[str]] = {
    "mod_decl": re.compile(r"^[ \t]*(pub(\([a-z]+\))?[ \t]+)?mod[ \t]+\w+[ \t]*;", re.M),
    "path_attr": re.compile(r"#\[\s*path\s*="),
    "include_macro": re.compile(r"\binclude_(str|bytes)?!\s*\("),
}
CRATE_PATH = re.compile(r"(?<![A-Za-z0-9_$])crate::")
INNER_CFG_LINE = re.compile(r"^\s*#!\[\s*cfg\s*\((.*)\)\s*\]\s*$")
MOD_COMMON = re.compile(r"^[ \t]*(pub(\([a-z]+\))?[ \t]+)?mod[ \t]+common[ \t]*;", re.M)
TEST_FN = re.compile(r"#\[test\][^\n]*\n(?:\s*#\[[^\n]*\n)*\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)")
SNAPSHOT_ASSERT = re.compile(r"\b(?:insta::)?assert_(?:snapshot|debug_snapshot|yaml_snapshot|json_snapshot|display_snapshot|toml_snapshot|ron_snapshot|csv_snapshot|compact_json_snapshot|compact_debug_snapshot|binary_snapshot)!")
FN_DEF = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*[<(]")
#: Insta settings that change where a snapshot lives (S4). One of them would
#: invalidate the mechanical rule for its module.
INSTA_PATH_SETTINGS = re.compile(r"\b(set_snapshot_path|snapshot_path|prepend_module_to_snapshot|set_prepend_module_to_snapshot|snapshot_suffix|set_snapshot_suffix)\b")


class ToolError(Exception):
    """An input could not be read or a subprocess failed (exit 2)."""


# ---------------------------------------------------------------------------
# Nextest filterset evaluation
# ---------------------------------------------------------------------------


class FilterError(ToolError):
    """A filterset this evaluator cannot parse or does not support."""


#: Predicates the evaluator understands, with Nextest's default matcher for
#: each. Anything else (`platform`, `deps`, `rdeps`, …) raises `FilterError`
#: rather than being guessed.
_PREDICATE_DEFAULT_MATCHER = {
    "test": "contains",
    "package": "glob",
    "binary_id": "glob",
    "binary": "glob",
    "kind": "equal",
    "all": None,
    "none": None,
}


@dataclass(frozen=True)
class FilterSubject:
    """What one filterset predicate can observe about one test."""

    package: str
    binary_id: str
    binary_name: str
    kind: str
    test: str


def _tokenize_filter(text: str) -> list[tuple[str, ...]]:
    tokens: list[tuple[str, ...]] = []
    index = 0
    while index < len(text):
        char = text[index]
        if char.isspace():
            index += 1
            continue
        if char in "()!&|+-":
            tokens.append(("op", char))
            index += 1
            continue
        match = re.match(r"[A-Za-z_][A-Za-z0-9_]*", text[index:])
        if not match:
            raise FilterError(f"unexpected character {char!r} at {index} in filter {text!r}")
        word = match.group(0)
        index += len(word)
        if word in ("not", "and", "or"):
            tokens.append(("op", {"not": "!", "and": "&", "or": "|"}[word]))
            continue
        rest = text[index:]
        stripped = rest.lstrip()
        if not stripped.startswith("("):
            raise FilterError(f"predicate {word!r} has no argument list in filter {text!r}")
        index += len(rest) - len(stripped) + 1
        while index < len(text) and text[index].isspace():
            index += 1
        if index < len(text) and text[index] == "/":
            end = index + 1
            while end < len(text) and text[end] != "/":
                end += 2 if text[end] == "\\" else 1
            if end >= len(text):
                raise FilterError(f"unterminated regex in filter {text!r}")
            argument = text[index:end + 1]
            index = end + 1
            while index < len(text) and text[index].isspace():
                index += 1
            if index >= len(text) or text[index] != ")":
                raise FilterError(f"expected ')' after regex in filter {text!r}")
        else:
            end = text.find(")", index)
            if end < 0:
                raise FilterError(f"unterminated predicate {word!r} in filter {text!r}")
            argument = text[index:end].strip()
            index = end
        index += 1
        tokens.append(("pred", word, argument))
    return tokens


class _FilterParser:
    """Recursive descent over Nextest's precedence: `!` > `&`/`-` > `|`/`+`."""

    def __init__(self, text: str) -> None:
        self.text = text
        self.tokens = _tokenize_filter(text)
        self.position = 0

    def peek(self) -> tuple[str, ...] | None:
        return self.tokens[self.position] if self.position < len(self.tokens) else None

    def take(self) -> tuple[str, ...]:
        token = self.peek()
        if token is None:
            raise FilterError(f"unexpected end of filter {self.text!r}")
        self.position += 1
        return token

    def parse(self) -> tuple:
        node = self.parse_or()
        if self.peek() is not None:
            raise FilterError(f"trailing input {self.peek()!r} in filter {self.text!r}")
        return node

    def parse_or(self) -> tuple:
        node = self.parse_and()
        while self.peek() in (("op", "|"), ("op", "+")):
            self.take()
            node = ("or", node, self.parse_and())
        return node

    def parse_and(self) -> tuple:
        node = self.parse_unary()
        while self.peek() in (("op", "&"), ("op", "-")):
            operator = self.take()[1]
            right = self.parse_unary()
            node = ("and", node, right) if operator == "&" else ("and", node, ("not", right))
        return node

    def parse_unary(self) -> tuple:
        token = self.take()
        if token == ("op", "!"):
            return ("not", self.parse_unary())
        if token == ("op", "("):
            node = self.parse_or()
            if self.take() != ("op", ")"):
                raise FilterError(f"unbalanced parentheses in filter {self.text!r}")
            return node
        if token[0] == "pred":
            name, argument = token[1], token[2]
            if name not in _PREDICATE_DEFAULT_MATCHER:
                raise FilterError(f"predicate {name}() is not supported by the projection evaluator (filter {self.text!r})")
            return ("pred", name, _parse_matcher(name, argument, self.text))
        raise FilterError(f"unexpected token {token!r} in filter {self.text!r}")


def _parse_matcher(predicate: str, argument: str, text: str) -> tuple[str, Any]:
    if predicate in ("all", "none"):
        if argument:
            raise FilterError(f"{predicate}() takes no argument in filter {text!r}")
        return ("none", None)
    if argument.startswith("/") and argument.endswith("/") and len(argument) >= 2:
        return ("regex", re.compile(argument[1:-1].replace("\\/", "/")))
    if argument[:1] in ("=", "~", "#"):
        kind = {"=": "equal", "~": "contains", "#": "glob"}[argument[0]]
        return (kind, argument[1:])
    return (_PREDICATE_DEFAULT_MATCHER[predicate], argument)


def parse_filter(text: str) -> tuple:
    return _FilterParser(text).parse()


def _match(matcher: tuple[str, Any], value: str) -> bool:
    kind, pattern = matcher
    if kind == "equal":
        return value == pattern
    if kind == "contains":
        return pattern in value
    if kind == "glob":
        return fnmatch.fnmatchcase(value, pattern)
    if kind == "regex":
        return pattern.search(value) is not None
    raise FilterError(f"unknown matcher kind {kind}")


def evaluate_filter(node: tuple, subject: FilterSubject) -> bool:
    kind = node[0]
    if kind == "or":
        return evaluate_filter(node[1], subject) or evaluate_filter(node[2], subject)
    if kind == "and":
        return evaluate_filter(node[1], subject) and evaluate_filter(node[2], subject)
    if kind == "not":
        return not evaluate_filter(node[1], subject)
    name, matcher = node[1], node[2]
    if name == "all":
        return True
    if name == "none":
        return False
    value = {
        "test": subject.test,
        "package": subject.package,
        "binary_id": subject.binary_id,
        "binary": subject.binary_name,
        "kind": subject.kind,
    }[name]
    return _match(matcher, value)


# ---------------------------------------------------------------------------
# Filter sources (the only place filter text enters the tool)
# ---------------------------------------------------------------------------


def _filter_env(include_slow: bool) -> dict[str, str]:
    env = {key: value for key, value in os.environ.items() if key not in ("BISCUIT_TEST_FILTER", "BISCUIT_L1_INCLUDE_SLOW")}
    if include_slow:
        env["BISCUIT_L1_INCLUDE_SLOW"] = "1"
    return env


def tier_filter(tier: str, package: str, *, include_slow: bool = False, repo: Path = REPO_ROOT) -> str:
    """The canonical expression, as `just _tier_filter` prints it."""
    completed = _run(["just", "_tier_filter", tier, package], cwd=repo, env=_filter_env(include_slow))
    return completed.stdout.strip()


def override_filters(repo: Path = REPO_ROOT) -> dict[str, str]:
    """`override-<profile>-<index>` → filter, read verbatim from `.config/nextest.toml`.

    Keyed by position so a rewritten filter (R5) is still the same selector.
    """
    config = tomllib.loads(_read_text(repo / ".config" / "nextest.toml"))
    selectors: dict[str, str] = {}
    for profile, body in config.get("profile", {}).items():
        for index, override in enumerate(body.get("overrides", [])):
            if "filter" in override:
                selectors[f"override-{profile}-{index}"] = override["filter"]
    return selectors


def package_ci_tests(manifest_path: Path) -> dict[str, Any]:
    manifest = tomllib.loads(_read_text(manifest_path))
    return manifest.get("package", {}).get("metadata", {}).get("ci", {}).get("tests", {})


def selector_filters(package: str, manifest_path: Path, repo: Path = REPO_ROOT) -> dict[str, str]:
    """Every selector a capture lists under: tiers, slow policy, overrides."""
    filters = {tier: tier_filter(tier, package, repo=repo) for tier in TIER_SELECTORS}
    if package_ci_tests(manifest_path).get("l1-include-slow") is True:
        filters[INCLUDE_SLOW_SELECTOR] = tier_filter("L1", package, include_slow=True, repo=repo)
    filters.update(override_filters(repo))
    return filters


def is_tier_selector(selector: str) -> bool:
    return not selector.startswith("override-")


# ---------------------------------------------------------------------------
# Listings and capture documents
# ---------------------------------------------------------------------------


def listing_digest(document: dict[str, Any]) -> str:
    """SHA-256 of a raw listing with its locality fields removed (S1)."""
    stripped = {key: value for key, value in document.items() if key != "rust-build-meta"}
    suites = {}
    for binary_id, suite in document.get("rust-suites", {}).items():
        suites[binary_id] = {key: value for key, value in suite.items() if key not in LOCALITY_SUITE_FIELDS}
    stripped["rust-suites"] = suites
    canonical = json.dumps(stripped, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode()).hexdigest()


def _verdict(case: dict[str, Any]) -> str:
    match = case.get("filter-match", {})
    if match.get("status") == "matches":
        return "matches"
    return str(match.get("reason", "unknown"))


def build_capture(
    *,
    host: str,
    package: str,
    features: tuple[str, ...],
    listings: dict[str, tuple[str | None, dict[str, Any]]],
    provenance: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Fold per-selector raw listings of one build into one capture document.

    Every listing of one build must name the same binaries, and each binary
    the same tests wherever it is listed. A difference means the build changed
    between listings, and the capture is refused.

    Nextest does not list a binary a filter rules out at binary level (a
    `package(...)` that cannot match): the suite is `skipped` with no test
    cases. Its tests take the verdict Nextest reports for a listed mismatch,
    `ignored` for an `#[ignore]`d test (which outranks `expression`) and
    `expression` otherwise. The skipped binaries are recorded per selector.
    """
    where = f"{host}/{package}/{feature_tag(features)}"
    suites: dict[str, dict[str, Any]] = {}
    selectors: dict[str, dict[str, Any]] = {}
    listed_tests: dict[str, tuple[str, set[str]]] = {}
    binaries: tuple[str, set[str]] | None = None
    for selector, (filter_text, document) in listings.items():
        if "rust-suites" not in document:
            raise ToolError(f"{where}/{selector}: not a nextest listing (no rust-suites)")
        skipped = []
        for binary_id, suite in document["rust-suites"].items():
            entry = suites.setdefault(binary_id, {
                "kind": suite.get("kind"),
                "binary_name": suite.get("binary-name"),
                "tests": {},
            })
            if suite.get("status") == "skipped":
                skipped.append(binary_id)
                continue
            names = set(suite.get("testcases", {}))
            reference = listed_tests.setdefault(binary_id, (selector, names))
            if names != reference[1]:
                raise ToolError(
                    f"{where}: listing {selector!r} names different tests in {binary_id} than {reference[0]!r} "
                    f"(only in {selector}: {sorted(names - reference[1])[:5]}; only in {reference[0]}: "
                    f"{sorted(reference[1] - names)[:5]}); the build changed between listings"
                )
            for test, case in suite.get("testcases", {}).items():
                record = entry["tests"].setdefault(test, {"ignored": bool(case.get("ignored")), "verdicts": {}})
                record["verdicts"][selector] = _verdict(case)
        present = set(document["rust-suites"])
        if binaries is None:
            binaries = (selector, present)
        elif present != binaries[1]:
            raise ToolError(f"{where}: listing {selector!r} names different binaries than {binaries[0]!r}; the build changed between listings")
        selectors[selector] = {"filter": filter_text, "listing_sha256": listing_digest(document), "skipped_binaries": sorted(skipped)}
    for selector, meta in selectors.items():
        for binary_id in meta["skipped_binaries"]:
            if binary_id not in listed_tests:
                raise ToolError(f"{where}: {binary_id} is skipped by every selector, so its tests are unknown")
            for record in suites[binary_id]["tests"].values():
                record["verdicts"][selector] = "ignored" if record["ignored"] else "expression"
    return {
        "kind": "consolidation-capture",
        "version": 1,
        "host": host,
        "package": package,
        "features": list(features),
        "feature_set": feature_tag(features),
        "provenance": provenance or {},
        "selectors": selectors,
        "suites": suites,
    }


def feature_tag(features: Iterable[str]) -> str:
    features = tuple(features)
    return "+".join(features) if features else "none"


def _read_maybe_gzip(path: Path) -> str:
    try:
        data = path.read_bytes()
    except OSError as error:
        raise ToolError(f"cannot read {path}: {error}") from error
    if path.suffix == ".gz":
        data = gzip.decompress(data)
    return data.decode("utf-8")


def write_gzip_json(path: Path, document: Any) -> None:
    """Deterministic gzip: no file name, zero mtime."""
    payload = (json.dumps(document, indent=1, sort_keys=True) + "\n").encode()
    path.write_bytes(gzip.compress(payload, compresslevel=9, mtime=0))


CaptureKey = tuple[str, str, str]  # (host, package, feature set)


def load_captures(paths: Iterable[Path]) -> dict[CaptureKey, dict[str, Any]]:
    """Capture documents, or Phase 1 raw listings folded into the same shape.

    Accepts `*.capture.json[.gz]` documents and raw listing files named
    `<host>__<package>__<feature set>__<selector>.json[.gz]`, as files or
    directories of them.
    """
    captures: dict[CaptureKey, dict[str, Any]] = {}
    raw: dict[CaptureKey, dict[str, tuple[str | None, dict[str, Any]]]] = defaultdict(dict)
    files: list[Path] = []
    for path in paths:
        path = Path(path)
        if path.is_dir():
            files.extend(sorted(p for p in path.iterdir() if p.name.endswith((".json", ".json.gz"))))
        elif path.exists():
            files.append(path)
        else:
            raise ToolError(f"capture path does not exist: {path}")
    for file in files:
        name = file.name.removesuffix(".gz").removesuffix(".json")
        if name.endswith(".capture"):
            document = json.loads(_read_maybe_gzip(file))
            if document.get("kind") != "consolidation-capture":
                raise ToolError(f"{file}: not a consolidation capture document")
            key = (document["host"], document["package"], document["feature_set"])
            if key in captures:
                raise ToolError(f"{file}: duplicate capture for {'/'.join(key)}")
            captures[key] = document
            continue
        parts = name.split("__")
        if len(parts) != 4:
            continue
        text = _read_maybe_gzip(file)
        if not text.strip():
            raise ToolError(f"{file}: empty listing; an absent listing is never read as zero tests")
        host, package, tag, selector = parts
        raw[(host, package, tag)][selector] = (None, json.loads(text))
    for key, listings in raw.items():
        if key in captures:
            raise ToolError(f"both a capture document and raw listings exist for {'/'.join(key)}")
        host, package, tag = key
        features = () if tag == "none" else tuple(tag.split("+"))
        captures[key] = build_capture(host=host, package=package, features=features, listings=listings,
                                      provenance={"source": "raw listings"})
    if not captures:
        raise ToolError(f"no captures found in {', '.join(str(p) for p in paths)}")
    return captures


# ---------------------------------------------------------------------------
# inventory
# ---------------------------------------------------------------------------


def cargo_metadata(repo: Path = REPO_ROOT) -> dict[str, Any]:
    completed = _run(["cargo", "metadata", "--no-deps", "--format-version", "1", "--color=never"], cwd=repo)
    return json.loads(completed.stdout)


def name_marker(package: str, name: str) -> str | None:
    for marker, _tier in MARKER_TIERS:
        if name.startswith(marker):
            return marker
    for marker in PACKAGE_MARKERS.get(package, ()):
        if name.startswith(marker):
            return marker
    return None


def marker_tier(package: str, name: str) -> str:
    marker = name_marker(package, name)
    return dict(MARKER_TIERS).get(marker or "", "L1")


def build_inventory(
    metadata: dict[str, Any],
    repo: Path,
    *,
    packages: Iterable[str] = PACKAGES,
    captures: dict[CaptureKey, dict[str, Any]] | None = None,
) -> tuple[dict[str, Any], list[str]]:
    """The merged per-package target table, and every consistency failure.

    Merges `cargo metadata` (what Cargo builds) with each manifest's
    `[[test]]` entries (the keys metadata omits). Neither is sufficient alone
    (spec §1), so a target present in one and absent from the other fails.
    """
    failures: list[str] = []
    packages = tuple(packages)
    members = set(metadata["workspace_members"])
    by_package = _tier_counts_by_binary(captures or {})

    workspace_test_targets = 0
    packages_with_tests = 0
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        count = sum(1 for target in package["targets"] if "test" in target["kind"])
        workspace_test_targets += count
        packages_with_tests += 1 if count else 0

    inventory: dict[str, Any] = {
        "generator": "scripts/ci/consolidation.py inventory",
        "workspace": {
            "workspace_members": len(members),
            "packages_with_test_targets": packages_with_tests,
            "integration_test_targets": workspace_test_targets,
        },
        "packages": {},
    }

    found = set()
    for package in metadata["packages"]:
        if package["name"] not in packages or package["id"] not in members:
            continue
        found.add(package["name"])
        manifest_path = Path(package["manifest_path"])
        crate_dir = manifest_path.parent
        manifest = tomllib.loads(_read_text(manifest_path))
        autotests = manifest.get("package", {}).get("autotests", True)
        declared = {entry["name"]: entry for entry in manifest.get("test", [])}
        tests_dir = crate_dir / "tests"
        metadata_tests = {t["name"]: t for t in package["targets"] if "test" in t["kind"]}

        for name, entry in declared.items():
            if name not in metadata_tests:
                failures.append(f"{package['name']}: [[test]] {name!r} is declared in {_rel(manifest_path, repo)} but cargo metadata does not report it")
            declared_path = crate_dir / entry["path"] if "path" in entry else None
            if declared_path is not None and not declared_path.is_file():
                failures.append(f"{package['name']}: [[test]] {name!r} path {_rel(declared_path, repo)} does not exist")
        for name, target in metadata_tests.items():
            source = Path(target["src_path"])
            if not source.is_file():
                failures.append(f"{package['name']}: test target {name!r} source {_rel(source, repo)} does not exist")
            if name not in declared:
                auto = source.parent == tests_dir or (source.name == "main.rs" and source.parent.parent == tests_dir)
                if not autotests or not auto:
                    failures.append(f"{package['name']}: test target {name!r} ({_rel(source, repo)}) is in cargo metadata but not declared in {_rel(manifest_path, repo)}")
        built_sources = {Path(t["src_path"]).resolve() for t in metadata_tests.values()}
        for root in sorted(tests_dir.glob("*/main.rs")):
            if root.resolve() not in built_sources:
                failures.append(f"{package['name']}: {_rel(root, repo)} is a nested crate root that no test target builds; its tests would silently never run")

        targets = []
        for target in sorted(metadata_tests.values(), key=lambda t: t["name"]):
            source = Path(target["src_path"])
            text = source.read_text(encoding="utf-8", errors="replace") if source.is_file() else ""
            entry = declared.get(target["name"], {})
            extra_keys = sorted(key for key in entry if key not in ("name", "path"))
            meta_features = sorted(target.get("required-features", []))
            manifest_features = sorted(entry.get("required-features", []))
            inner_cfgs = [m.group(1) for line in text.splitlines() if (m := INNER_CFG_LINE.match(line))]
            binary_id = f"{package['name']}::{target['name']}"

            tiers_by_feature_set: dict[str, dict[str, int]] = {}
            test_names: set[str] = set()
            ignored: set[str] = set()
            for feature_set, binaries in sorted(by_package.get(package["name"], {}).items()):
                if binary_id not in binaries:
                    continue
                counts, names, ignored_names = binaries[binary_id]
                tiers_by_feature_set[feature_set] = counts
                test_names.update(names)
                ignored.update(ignored_names)

            marker = name_marker(package["name"], target["name"])
            # The Phase 1 heuristic, kept for the record: a marker-named target
            # is alias-free if every test already carries the marker. `plan`
            # decides aliases by projecting every filter instead.
            alias_required = None
            alias_basis = None
            if marker is not None:
                names = test_names or set(TEST_FN.findall(text))
                alias_basis = "macos-listing" if test_names else "source-scan (platform-absent on macOS; confirm from on-host listing)"
                if names:
                    alias_required = any(not re.search(rf"(^|::){marker}", test) for test in names)

            targets.append({
                "name": target["name"],
                "src_path": _rel(source, repo),
                "nested_root": source.parent != tests_dir,
                "declared_in_manifest": target["name"] in declared,
                "required_features": meta_features,
                "required_features_manifest": manifest_features,
                "required_features_agree": meta_features == manifest_features or not declared.get(target["name"]),
                "harness": entry.get("harness", True),
                "edition": target.get("edition"),
                "target_wide_keys": extra_keys,
                "name_marker": marker,
                "module_name_alias_required": alias_required,
                "module_name_alias_basis": alias_basis,
                "inner_cfg": inner_cfgs,
                "declares_mod_common": bool(MOD_COMMON.search(text)),
                "listed_tests": len(test_names),
                "ignored_tests": len(ignored),
                "selected_by_tier": tiers_by_feature_set,
            })

        nested_roots = sorted(_rel(p, repo) for p in tests_dir.glob("*/main.rs"))
        inventory["packages"][package["name"]] = {
            "manifest": _rel(manifest_path, repo),
            "autotests": autotests,
            "features": sorted(manifest.get("features", {})),
            "ci_tests": manifest.get("package", {}).get("metadata", {}).get("ci", {}).get("tests", {}),
            "top_level_test_files": len(list(tests_dir.glob("*.rs"))),
            "nested_main_rs": nested_roots,
            "test_targets": targets,
            "bench_targets": [
                {"name": b["name"], "harness": b.get("harness", True)} for b in manifest.get("bench", [])
            ],
        }
    for missing in sorted(set(packages) - found):
        failures.append(f"{missing}: not a workspace member reported by cargo metadata")
    return inventory, failures


INVENTORY_TIERS = ("L1", "L2", "L3", "browser", "real")


def _tier_counts_by_binary(captures: dict[CaptureKey, dict[str, Any]]) -> dict[str, dict[str, dict[str, tuple]]]:
    """{package: {feature set: {binary id: (tier counts, test names, ignored names)}}}.

    Tier placement is one host's observation, so captures from more than one
    host are refused rather than silently overwritten.
    """
    hosts = {key[0] for key in captures}
    if len(hosts) > 1:
        raise ToolError(f"inventory reads one host's captures; got {sorted(hosts)}")
    result: dict[str, dict[str, dict[str, tuple]]] = defaultdict(dict)
    for (_host, package, feature_set), capture in sorted(captures.items()):
        binaries = {}
        for binary_id, suite in capture["suites"].items():
            if suite.get("kind") != "test":
                continue
            counts = {tier: 0 for tier in INVENTORY_TIERS}
            names = set(suite["tests"])
            ignored = set()
            for test, record in suite["tests"].items():
                for tier in INVENTORY_TIERS:
                    if record["verdicts"].get(tier) == "matches":
                        counts[tier] += 1
                if record["ignored"]:
                    ignored.add(test)
            binaries[binary_id] = (counts, names, ignored)
        result[package][feature_set] = binaries
    return result


def contract_label(target: dict[str, Any]) -> str:
    features = ",".join(target["required_features"]) or "-"
    harness = "std" if target["harness"] else "custom"
    wide = ",".join(k for k in target["target_wide_keys"] if k != "required-features") or "-"
    return f"features={features} harness={harness} other-keys={wide}"


def render_inventory_markdown(inventory: dict[str, Any]) -> str:
    lines = [
        "# Test-target inventory",
        "",
        f"Generated by `{inventory['generator']}` from `cargo metadata --no-deps`, each",
        "package `Cargo.toml`, and the supplied captures.",
        "",
        "| Package | Top-level `tests/*.rs` | Test targets | Nested `main.rs` roots | `autotests` | Declared `[[test]]` | `mod common;` files |",
        "|---|---:|---:|---|---|---:|---:|",
    ]
    for name, package in inventory["packages"].items():
        targets = package["test_targets"]
        lines.append(
            f"| `{name}` | {package['top_level_test_files']} | {len(targets)} | "
            f"{', '.join(f'`{p}`' for p in package['nested_main_rs']) or '—'} | {package['autotests']} | "
            f"{sum(1 for t in targets if t['declared_in_manifest'])} | "
            f"{sum(1 for t in targets if t['declares_mod_common'])} |"
        )
    lines += ["", "## Execution contracts", ""]
    for name, package in inventory["packages"].items():
        groups: dict[str, list[str]] = defaultdict(list)
        for target in package["test_targets"]:
            groups[contract_label(target)].append(target["name"])
        lines += [f"### `{name}`", "", "| Contract | Targets |", "|---|---:|"]
        lines += [f"| `{key}` | {len(names)} |" for key, names in sorted(groups.items())]
        lines.append("")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# plan
# ---------------------------------------------------------------------------


@dataclass
class PlanResult:
    manifest: dict[str, Any]
    failures: list[str] = field(default_factory=list)


def _subject(package: str, binary: str, test: str) -> FilterSubject:
    return FilterSubject(package=package, binary_id=f"{package}::{binary}", binary_name=binary, kind="test", test=test)


def check_evaluator_agreement(
    package: str,
    captures: dict[CaptureKey, dict[str, Any]],
    parsed: dict[str, tuple],
    filters: dict[str, str],
) -> list[str]:
    """Every Nextest verdict in the captures, re-derived by the evaluator.

    Ignored tests are skipped: Nextest reports them as `ignored` without
    saying what the expression decided.
    """
    failures = []
    for (host, capture_package, feature_set), capture in sorted(captures.items()):
        if capture_package != package:
            continue
        for selector, meta in capture["selectors"].items():
            if selector not in parsed:
                continue
            recorded = meta.get("filter")
            if recorded is not None and recorded != filters[selector]:
                failures.append(f"{host}/{package}/{feature_set}: selector {selector} was captured under {recorded!r}, but the current filter is {filters[selector]!r}; recapture before planning")
                continue
            node = parsed[selector]
            for binary_id, suite in capture["suites"].items():
                package_name, _, binary = binary_id.partition("::")
                for test, record in suite["tests"].items():
                    verdict = record["verdicts"].get(selector)
                    if verdict not in ("matches", "expression"):
                        continue
                    subject = FilterSubject(package_name, binary_id, suite.get("binary_name") or binary, suite.get("kind") or "test", test)
                    if evaluate_filter(node, subject) != (verdict == "matches"):
                        failures.append(f"{host}/{package}/{feature_set}: the projection evaluator disagrees with nextest on {binary_id} {test} under {selector} ({filters[selector]!r}); nextest said {verdict}")
    return failures


def plan_package(
    package: str,
    inventory: dict[str, Any],
    captures: dict[CaptureKey, dict[str, Any]],
    filters: dict[str, str],
    repo: Path = REPO_ROOT,
) -> PlanResult:
    """Group targets into consolidated targets and assign module names.

    A module keeps its former target name unless that name changes some
    test's verdict under some filter (R2). A change every module name would
    cause (an exact-name override) is recorded as a required override
    rewrite instead (R5).
    """
    failures: list[str] = []
    if package not in inventory["packages"]:
        raise ToolError(f"{package} is not in the inventory")
    record = inventory["packages"][package]
    manifest_rel = record["manifest"]
    crate_dir = str(Path(manifest_rel).parent.as_posix())
    parsed = {selector: parse_filter(text) for selector, text in filters.items()}
    failures += check_evaluator_agreement(package, captures, parsed, filters)

    tests_by_binary: dict[str, set[str]] = defaultdict(set)
    for (_host, capture_package, _fs), capture in captures.items():
        if capture_package != package:
            continue
        for binary_id, suite in capture["suites"].items():
            if suite.get("kind") == "test":
                tests_by_binary[binary_id].update(suite["tests"])

    contracts: dict[tuple, list[dict[str, Any]]] = defaultdict(list)
    for target in record["test_targets"]:
        others = [k for k in target["target_wide_keys"] if k not in ("required-features", "harness")]
        if not target["harness"]:
            failures.append(f"{package}: test target {target['name']!r} sets harness = false; a custom-harness target cannot become a module and stays a separate target (R3)")
            continue
        if others:
            failures.append(f"{package}: test target {target['name']!r} sets {', '.join(others)}; a target-wide key needs a recorded decision before it can be consolidated (R3)")
            continue
        tier = marker_tier(package, target["name"])
        contracts[(tier, tuple(target["required_features"]))].append(target)

    names: dict[tuple, str] = {}
    by_tier: dict[str, list[tuple]] = defaultdict(list)
    for key in contracts:
        by_tier[key[0]].append(key)
    for tier, keys in by_tier.items():
        base = TIER_TARGET_NAMES[tier]
        for key in keys:
            features = key[1]
            if len(keys) == 1 or not features:
                names[key] = base
            else:
                names[key] = base + "-" + "-".join(f.removesuffix("-tests") for f in features)
    if len(set(names.values())) != len(names):
        failures.append(f"{package}: consolidated target names collide: {sorted(names.values())}")

    targets_out = []
    modules_out = []
    rewrites: dict[str, dict[str, Any]] = {}
    for key in sorted(contracts, key=lambda k: names[k]):
        target_name = names[key]
        crate_prefix = f"{crate_dir}/tests/{target_name}"
        module_names: list[str] = []
        for old in sorted(contracts[key], key=lambda t: t["name"]):
            old_name = old["name"]
            binary_id = f"{package}::{old_name}"
            tests = sorted(tests_by_binary.get(binary_id, set()))
            basis = "listing"
            if not tests:
                source = repo / old["src_path"]
                text = source.read_text(encoding="utf-8", errors="replace") if source.is_file() else ""
                tests = sorted(set(TEST_FN.findall(text)))
                basis = "source-scan" if tests else "none"

            candidate = old_name.replace("-", "_")
            alias_reason: list[str] = []
            if candidate != old_name:
                alias_reason.append("the target name is not a Rust identifier")
            hazards, moved = _project(package, old_name, target_name, candidate, tests, parsed)
            if hazards:
                marker = name_marker(package, old_name)
                alias_reason += hazards[:5]
                if marker is None:
                    failures.append(f"{package}: module name {candidate!r} changes verdicts ({'; '.join(hazards[:3])}) and carries no marker prefix to strip; choose an alias by hand")
                else:
                    candidate = old_name[len(marker):].replace("-", "_")
                    if candidate in module_names or candidate in RESERVED_MODULES:
                        candidate += "_module"
                    hazards, moved = _project(package, old_name, target_name, candidate, tests, parsed)
                    if hazards:
                        failures.append(f"{package}: alias {candidate!r} for {old_name!r} still changes verdicts: {'; '.join(hazards[:3])}")
            if candidate in RESERVED_MODULES:
                failures.append(f"{package}: module name {candidate!r} (from {old_name!r}) collides with a crate-root module the consolidated root declares itself")
            if candidate in module_names:
                failures.append(f"{package}: two old targets project to module {candidate!r} in {target_name!r}")
            if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", candidate):
                failures.append(f"{package}: module name {candidate!r} is not a Rust identifier")
            module_names.append(candidate)
            for selector, test in moved:
                rewrite = rewrites.setdefault(selector, {"selector": selector, "filter": filters[selector], "tests": []})
                rewrite["tests"].append({"old": f"{binary_id} {test}", "new": f"{package}::{target_name} {candidate}::{test}"})

            nested = bool(old["nested_root"])
            modules_out.append({
                "old_target": old_name,
                "old_binary_id": binary_id,
                "old_path": old["src_path"],
                "nested_root": nested,
                "target": target_name,
                "binary_id": f"{package}::{target_name}",
                "module": candidate,
                "new_path": f"{crate_prefix}/{candidate}/mod.rs" if nested else f"{crate_prefix}/{candidate}.rs",
                "alias": {"from": old_name, "reason": alias_reason} if candidate != old_name else None,
                "inner_cfg": old["inner_cfg"],
                "test_basis": basis,
                "tests": [{"old": test, "new": f"{candidate}::{test}"} for test in tests],
            })
        targets_out.append({
            "name": target_name,
            "path": f"{crate_prefix}/main.rs",
            "tier": key[0],
            "required_features": list(key[1]),
            "harness": True,
            "modules": module_names,
        })

    for rewrite in rewrites.values():
        exact = re.fullmatch(r"\s*test\(\s*=([^)]*)\)\s*", rewrite["filter"])
        if exact and len({t["new"] for t in rewrite["tests"]}) == 1:
            rewrite["suggested_filter"] = f"test(={rewrite['tests'][0]['new'].split(' ', 1)[1]})"
        else:
            rewrite["suggested_filter"] = None

    manifest = {
        "kind": "consolidation-manifest",
        "version": 1,
        "package": package,
        "manifest": manifest_rel,
        "crate_dir": crate_dir,
        "generator": "scripts/ci/consolidation.py plan",
        "filters": filters,
        "targets": targets_out,
        "modules": modules_out,
        "override_rewrites": sorted(rewrites.values(), key=lambda r: r["selector"]),
        "additions": [],
        "dispositions": [],
    }
    return PlanResult(manifest=manifest, failures=failures)


def _project(
    package: str,
    old_binary: str,
    new_binary: str,
    module: str,
    tests: list[str],
    parsed: dict[str, tuple],
) -> tuple[list[str], list[tuple[str, str]]]:
    """(name hazards, name-independent override changes) for one module name."""
    hazards: list[str] = []
    moved: list[tuple[str, str]] = []
    for selector, node in parsed.items():
        for test in tests:
            before = evaluate_filter(node, _subject(package, old_binary, test))
            after = evaluate_filter(node, _subject(package, new_binary, f"{module}::{test}"))
            if before == after:
                continue
            probe = evaluate_filter(node, _subject(package, new_binary, f"{NEUTRAL_PROBE_MODULE}::{test}"))
            if not is_tier_selector(selector) and after == probe:
                moved.append((selector, test))
            else:
                hazards.append(f"{selector}: {test} {'matches' if before else 'mismatches'} before, {'matches' if after else 'mismatches'} as {module}::{test}")
    return hazards, moved


# ---------------------------------------------------------------------------
# capture
# ---------------------------------------------------------------------------


def host_name() -> str:
    return platform.system().lower()


def capture_package(
    package: str,
    out_dir: Path,
    *,
    feature_sets: Iterable[tuple[str, ...]] | None = None,
    raw_dir: Path | None = None,
    repo: Path = REPO_ROOT,
    log: Callable[[str], None] = lambda line: print(line, file=sys.stderr),
) -> list[Path]:
    """List every selector for every feature set; write one capture per set."""
    metadata = cargo_metadata(repo)
    manifest_path = next((Path(p["manifest_path"]) for p in metadata["packages"] if p["name"] == package), None)
    if manifest_path is None:
        raise ToolError(f"{package} is not a workspace package")
    manifest = tomllib.loads(_read_text(manifest_path))
    declared_features = set(manifest.get("features", {}))
    sets = tuple(feature_sets) if feature_sets is not None else PACKAGE_FEATURE_SETS.get(package)
    if sets is None:
        raise ToolError(f"no feature sets are known for {package}; pass --feature-set")
    for features in sets:
        unknown = sorted(set(features) - declared_features)
        if unknown:
            raise ToolError(f"{package}: feature set {feature_tag(features)} names undeclared features {unknown}")
    ci_union = set(package_ci_tests(manifest_path).get("features", []))
    if feature_sets is None and ci_union not in [set(s) for s in sets]:
        raise ToolError(f"{package}: the CI feature union {sorted(ci_union)} is not one of the captured feature sets; update PACKAGE_FEATURE_SETS")

    filters = selector_filters(package, manifest_path, repo)
    host = host_name()
    provenance = {
        "revision": _run(["git", "rev-parse", "HEAD"], cwd=repo).stdout.strip(),
        "crate_tree_dirty": bool(_run(["git", "status", "--porcelain", "--", _rel(manifest_path.parent, repo)], cwd=repo).stdout.strip()),
        "nextest_version": _run(["cargo", "nextest", "--version"], cwd=repo).stdout.splitlines()[0].strip(),
        "environment": {"BISCUIT_TEST_FILTER": "unset", "BISCUIT_L1_INCLUDE_SLOW": f"set only for {INCLUDE_SLOW_SELECTOR}"},
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    written = []
    env = _filter_env(False)
    for features in sets:
        feature_args = ["--features", ",".join(features)] if features else []
        base = ["cargo", "nextest", "list", "--color=never", "-p", package, *feature_args, "--message-format", "json"]
        tag = feature_tag(features)
        log(f"capture {host}/{package}/{tag}: building")
        _run(base, cwd=repo, env=env)
        cache: dict[str, dict[str, Any]] = {}
        listings: dict[str, tuple[str | None, dict[str, Any]]] = {}
        for selector, text in filters.items():
            if text not in cache:
                cache[text] = json.loads(_run([*base, "-E", text], cwd=repo, env=env).stdout)
            listings[selector] = (text, cache[text])
            if raw_dir is not None:
                raw_dir.mkdir(parents=True, exist_ok=True)
                write_gzip_json(raw_dir / f"{host}__{package}__{tag}__{selector}.json.gz", cache[text])
        capture = build_capture(host=host, package=package, features=features, listings=listings, provenance=provenance)
        path = out_dir / f"{host}__{package}__{tag}.capture.json.gz"
        write_gzip_json(path, capture)
        log(f"capture {host}/{package}/{tag}: {len(listings)} selectors, {len(cache)} distinct listings → {path}")
        written.append(path)
    return written


# ---------------------------------------------------------------------------
# compare
# ---------------------------------------------------------------------------

SETS = ("selected", "excluded", "ignored")


def _normalizer(manifest: dict[str, Any] | None) -> Callable[[str, str], tuple[str, str] | None]:
    """Map an after identity back to its before identity; `None` if unmapped."""
    if manifest is None:
        return lambda binary_id, test: (binary_id, test)
    modules = {(m["binary_id"], m["module"]): m["old_binary_id"] for m in manifest["modules"]}
    consolidated = {t for t, _ in modules}

    def normalize(binary_id: str, test: str) -> tuple[str, str] | None:
        if binary_id not in consolidated:
            return (binary_id, test)
        module, separator, rest = test.partition("::")
        old = modules.get((binary_id, module))
        if old is None or not separator:
            return None
        return (old, rest)

    return normalize


def _identity_sets(capture: dict[str, Any], selector: str) -> dict[str, set[str]]:
    sets = {name: set() for name in SETS}
    sets["present"] = set()
    for binary_id, suite in capture["suites"].items():
        for test, record in suite["tests"].items():
            identity = f"{binary_id} {test}"
            sets["present"].add(identity)
            verdict = record["verdicts"].get(selector)
            if verdict == "matches":
                sets["selected"].add(identity)
            elif verdict == "expression":
                sets["excluded"].add(identity)
            elif verdict == "ignored":
                sets["ignored"].add(identity)
            else:
                sets.setdefault("unknown", set()).add(identity)
    return sets


def _present(capture: dict[str, Any]) -> set[str]:
    return {f"{binary_id} {test}" for binary_id, suite in capture["suites"].items() for test in suite["tests"]}


def normalize_capture(
    capture: dict[str, Any],
    manifest: dict[str, Any] | None,
    where: str,
    failures: list[str],
) -> dict[str, Any]:
    """The after capture with every identity mapped back to its before form."""
    normalize = _normalizer(manifest)
    additions = {(f"{manifest['package']}::{a['target']}", a["test"]) for a in (manifest or {}).get("additions", [])}
    seen_additions = set()
    suites: dict[str, dict[str, Any]] = {}
    for binary_id, suite in capture["suites"].items():
        for test, record in suite["tests"].items():
            if (binary_id, test) in additions:
                seen_additions.add((binary_id, test))
                continue
            mapped = normalize(binary_id, test)
            if mapped is None:
                failures.append(f"{where}: {binary_id} {test} is not mapped by the migration manifest (no module entry for its first path segment)")
                continue
            old_binary, old_test = mapped
            target = suites.setdefault(old_binary, {"kind": suite.get("kind"), "binary_name": old_binary.partition("::")[2], "tests": {}})
            if old_test in target["tests"]:
                failures.append(f"{where}: two after-state tests normalize to the same identity {old_binary} {old_test}")
                continue
            target["tests"][old_test] = record
    for binary_id, test in sorted(additions - seen_additions):
        failures.append(f"{where}: the manifest declares addition {binary_id} {test}, but the after capture does not list it")
    return {**capture, "suites": suites}


def compare_captures(
    before: dict[CaptureKey, dict[str, Any]],
    after: dict[CaptureKey, dict[str, Any]],
    manifests: dict[str, dict[str, Any]],
    *,
    packages: Iterable[str] | None = None,
    common_selectors: bool = False,
    require_identical_digests: bool = False,
) -> dict[str, Any]:
    failures: list[str] = []
    notes: list[str] = []
    scope = sorted(set(packages) if packages else {key[1] for key in before})
    report: dict[str, Any] = {"kind": "consolidation-comparison", "version": 1, "packages": {}}
    for package in scope:
        manifest = manifests.get(package)
        before_keys = {key for key in before if key[1] == package}
        hosts = sorted({key[0] for key in before_keys})
        after_keys = {key for key in after if key[1] == package and key[0] in hosts}
        if not before_keys:
            failures.append(f"{package}: no before capture")
        for key in sorted(before_keys - after_keys):
            failures.append(f"{'/'.join(key)}: before capture has no after capture (a missing listing is never read as zero tests)")
        for key in sorted(after_keys - before_keys):
            failures.append(f"{'/'.join(key)}: after capture has no before capture")
        for extra in sorted({key[0] for key in after if key[1] == package} - set(hosts)):
            notes.append(f"{package}: after captures for host {extra} have no before side and were not compared")
        cells = []
        normalized_after: dict[CaptureKey, dict[str, Any]] = {}
        for key in sorted(before_keys & after_keys):
            where = "/".join(key)
            b, a = before[key], normalize_capture(after[key], manifest, where, failures)
            normalized_after[key] = a
            b_selectors, a_selectors = set(b["selectors"]), set(a["selectors"])
            selectors = sorted(b_selectors & a_selectors)
            for selector in sorted(b_selectors ^ a_selectors):
                side = "before" if selector in b_selectors else "after"
                message = f"{where}: selector {selector} was captured on the {side} side only"
                if common_selectors:
                    notes.append(message + " (not compared: --common-selectors)")
                else:
                    failures.append(message)
            cell = {"host": key[0], "feature_set": key[2], "selectors": {}}
            for selector in selectors:
                filter_before = b["selectors"][selector].get("filter")
                filter_after = a["selectors"][selector].get("filter")
                entry: dict[str, Any] = {"filter_before": filter_before, "filter_after": filter_after, "sets": {}}
                if filter_before is not None and filter_after is not None and filter_before != filter_after:
                    if is_tier_selector(selector):
                        failures.append(f"{where}: tier selector {selector} changed its filter ({filter_before!r} → {filter_after!r}); a tier expression never changes in a migration")
                    else:
                        notes.append(f"{where}: override {selector} was rewritten ({filter_before!r} → {filter_after!r})")
                b_sets, a_sets = _identity_sets(b, selector), _identity_sets(a, selector)
                for side, sets in (("before", b_sets), ("after", a_sets)):
                    if sets.get("unknown"):
                        failures.append(f"{where}: {selector} has verdicts this tool does not classify on the {side} side: {sorted(sets['unknown'])[:3]}")
                for name in ("present", *SETS):
                    lost = sorted(b_sets[name] - a_sets[name])
                    gained = sorted(a_sets[name] - b_sets[name])
                    entry["sets"][name] = {"before": len(b_sets[name]), "after": len(a_sets[name]), "lost": lost, "gained": gained}
                    if lost or gained:
                        failures.append(f"{where}: {selector} {name} differs — lost {len(lost)} {lost[:3]}, gained {len(gained)} {gained[:3]}")
                digest_before = b["selectors"][selector].get("listing_sha256")
                digest_after = after[key]["selectors"][selector].get("listing_sha256")
                entry["listing_digest_identical"] = digest_before == digest_after
                if require_identical_digests and digest_before != digest_after:
                    failures.append(f"{where}: {selector} listing digest differs ({digest_before} → {digest_after}) although identical digests were required")
                cell["selectors"][selector] = entry
            cells.append(cell)
        report["packages"][package] = {
            "manifest": bool(manifest),
            "cells": cells,
            "platform_absent": _platform_absent(package, before, normalized_after, before_keys & after_keys, failures),
        }
    report["failures"] = failures
    report["notes"] = notes
    report["verdict"] = "identical" if not failures else "different"
    return report


def _platform_absent(
    package: str,
    before: dict[CaptureKey, dict[str, Any]],
    after: dict[CaptureKey, dict[str, Any]],
    keys: set[CaptureKey],
    failures: list[str],
) -> dict[str, Any]:
    """Per feature set: identities other hosts list and this host does not.

    One host cannot observe its own platform-absent set (S1 §3), so a single
    host reports `not-evaluated`, never an empty set.
    """
    result: dict[str, Any] = {}
    for feature_set in sorted({key[2] for key in keys}):
        hosts = sorted(key[0] for key in keys if key[2] == feature_set)
        if len(hosts) < 2:
            result[feature_set] = {"status": "not-evaluated", "hosts": hosts, "reason": "captures from one host only; platform-absent needs the other hosts' captures"}
            continue
        per_host = {}
        for side, captures in (("before", before), ("after", after)):
            present = {host: _present(captures[(host, package, feature_set)]) for host in hosts}
            union = set().union(*present.values())
            per_host[side] = {host: union - present[host] for host in hosts}
        diffs = {}
        for host in hosts:
            lost = sorted(per_host["before"][host] - per_host["after"][host])
            gained = sorted(per_host["after"][host] - per_host["before"][host])
            diffs[host] = {"before": len(per_host["before"][host]), "after": len(per_host["after"][host]), "lost": lost, "gained": gained}
            if lost or gained:
                failures.append(f"{host}/{package}/{feature_set}: platform-absent differs — lost {lost[:3]}, gained {gained[:3]}")
        result[feature_set] = {"status": "evaluated", "hosts": hosts, "per_host": diffs}
    return result


def render_comparison_markdown(report: dict[str, Any]) -> str:
    lines = [f"# Identity comparison: {report['verdict']}", ""]
    for package, body in report["packages"].items():
        lines += [f"## `{package}`", "", f"Migration manifest applied: {'yes' if body['manifest'] else 'no (identity mapping)'}", ""]
        lines += ["| Host | Feature set | Selector | Present | Selected | Excluded | Ignored | Differences |", "|---|---|---|---:|---:|---:|---:|---|"]
        for cell in body["cells"]:
            for selector, entry in cell["selectors"].items():
                sets = entry["sets"]
                diff = sum(len(s["lost"]) + len(s["gained"]) for s in sets.values())
                lines.append(
                    f"| {cell['host']} | `{cell['feature_set']}` | `{selector}` | {sets['present']['after']} | "
                    f"{sets['selected']['after']} | {sets['excluded']['after']} | {sets['ignored']['after']} | "
                    f"{'none' if not diff else f'**{diff} identities**'} |"
                )
        lines += ["", "Platform-absent:", ""]
        for feature_set, status in body["platform_absent"].items():
            lines.append(f"- `{feature_set}`: {status['status']} (hosts: {', '.join(status['hosts'])})" + (f" — {status['reason']}" if "reason" in status else ""))
        lines.append("")
    if report["failures"]:
        lines += ["## Failures", ""] + [f"- {failure}" for failure in report["failures"]] + [""]
    if report["notes"]:
        lines += ["## Notes", ""] + [f"- {note}" for note in report["notes"]] + [""]
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Rust source scanning (check-attributes)
# ---------------------------------------------------------------------------


def _skip_string(text: str, index: int) -> int:
    """Index just past the string or char literal starting at `index`."""
    if text.startswith("r", index) or text.startswith("br", index):
        start = index + (2 if text[index] == "b" else 1)
        hashes = 0
        while start + hashes < len(text) and text[start + hashes] == "#":
            hashes += 1
        if start + hashes < len(text) and text[start + hashes] == '"':
            terminator = '"' + "#" * hashes
            end = text.find(terminator, start + hashes + 1)
            return len(text) if end < 0 else end + len(terminator)
        return index + 1
    quote = text[index]
    if quote == "'":
        # A char literal ('x', '\n', '\u{1F600}') versus a lifetime ('a).
        match = re.match(r"'(\\u\{[0-9a-fA-F]+\}|\\.|[^\\'])'", text[index:])
        return index + len(match.group(0)) if match else index + 1
    position = index + 1
    while position < len(text):
        if text[position] == "\\":
            position += 2
            continue
        if text[position] == quote:
            return position + 1
        position += 1
    return len(text)


def _skip_trivia(text: str, index: int) -> int:
    """Skip whitespace and comments (including doc comments; nesting-aware)."""
    while index < len(text):
        if text[index].isspace():
            index += 1
        elif text.startswith("//", index):
            end = text.find("\n", index)
            index = len(text) if end < 0 else end + 1
        elif text.startswith("/*", index):
            depth, index = 1, index + 2
            while index < len(text) and depth:
                if text.startswith("/*", index):
                    depth, index = depth + 1, index + 2
                elif text.startswith("*/", index):
                    depth, index = depth - 1, index + 2
                else:
                    index += 1
        else:
            break
    return index


def _balanced(text: str, index: int, open_char: str, close_char: str) -> int:
    """Index just past the `close_char` matching the `open_char` at `index`."""
    depth = 0
    while index < len(text):
        char = text[index]
        if char == '"' or (char in "rb" and re.match(r'b?r#*"', text[index:])) or (char == "'" and re.match(r"'(\\.|[^\\'])'", text[index:])):
            index = _skip_string(text, index)
            continue
        if char == open_char:
            depth += 1
        elif char == close_char:
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    raise ToolError(f"unbalanced {open_char}{close_char} in Rust source")


def normalize_attribute(body: str) -> str:
    """Attribute text with whitespace outside string literals removed."""
    out = []
    index = 0
    while index < len(body):
        char = body[index]
        if char == '"':
            end = _skip_string(body, index)
            out.append(body[index:end])
            index = end
        elif char.isspace():
            index += 1
        else:
            out.append(char)
            index += 1
    return "".join(out)


def leading_inner_attributes(text: str) -> list[str]:
    """Every `#![…]` in the file's leading doc/attribute block, normalized.

    Finds attributes below a long module doc and multi-line attributes, not
    only a single-line one on line 1 (S2).
    """
    attributes = []
    index = 0
    if text.startswith("\ufeff"):
        index = 1
    while True:
        index = _skip_trivia(text, index)
        if not text.startswith("#!", index):
            break
        bracket = _skip_trivia(text, index + 2)
        if not text.startswith("[", bracket):
            break  # a shebang line, not an attribute
        end = _balanced(text, bracket, "[", "]")
        attributes.append(normalize_attribute(text[bracket + 1:end - 1]))
        index = end
    return attributes


def module_declarations(text: str) -> dict[str, list[str]]:
    """`mod name;` declarations at the top level, with their outer attributes."""
    declarations: dict[str, list[str]] = {}
    pending: list[str] = []
    index = 0
    while True:
        index = _skip_trivia(text, index)
        if index >= len(text):
            break
        if text.startswith("#!", index) or text.startswith("#[", index):
            inner = text.startswith("#!", index)
            bracket = text.find("[", index)
            end = _balanced(text, bracket, "[", "]")
            if not inner:
                pending.append(normalize_attribute(text[bracket + 1:end - 1]))
            index = end
            continue
        start = index
        depth = 0
        while index < len(text):
            char = text[index]
            if char == '"' or (char in "rb" and re.match(r'b?r#*"', text[index:])) or (char == "'" and re.match(r"'(\\.|[^\\'])'", text[index:])):
                index = _skip_string(text, index)
                continue
            if text.startswith("//", index) or text.startswith("/*", index):
                index = _skip_trivia(text, index)
                continue
            if char in "{([":
                depth += 1
            elif char in "})]":
                depth -= 1
                if depth == 0 and char == "}":
                    index += 1
                    break
            elif char == ";" and depth == 0:
                index += 1
                break
            index += 1
        item = " ".join(text[start:index].split())
        match = re.fullmatch(r"(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;", item)
        if match:
            declarations[match.group(1)] = pending
        pending = []
    return declarations


def _attribute_name(body: str) -> str:
    match = re.match(r"(?:unsafe\()?([A-Za-z_][A-Za-z0-9_:]*)", body)
    return match.group(1) if match else ""


def _line_of(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def _detector_hits(text: str, detectors: dict[str, re.Pattern[str]]) -> dict[str, list[int]]:
    hits = {}
    for name, pattern in detectors.items():
        lines = [_line_of(text, m.start()) for m in pattern.finditer(text)]
        if lines:
            hits[name] = lines
    return hits


class SourceReader:
    """Reads repository files as of the before state: a git revision or a directory."""

    def __init__(self, *, rev: str | None = None, root: Path | None = None, repo: Path = REPO_ROOT) -> None:
        if (rev is None) == (root is None):
            raise ToolError("exactly one of a before revision or a before directory is required")
        self.rev, self.root, self.repo = rev, root, repo

    def read(self, path: str) -> str | None:
        if self.root is not None:
            file = self.root / path
            return file.read_text(encoding="utf-8", errors="replace") if file.is_file() else None
        completed = subprocess.run(["git", "show", f"{self.rev}:{path}"], cwd=self.repo, capture_output=True)
        return completed.stdout.decode("utf-8", errors="replace") if completed.returncode == 0 else None

    def list_files(self, prefix: str) -> list[str]:
        if self.root is not None:
            base = self.root / prefix
            return sorted(p.relative_to(self.root).as_posix() for p in base.rglob("*") if p.is_file()) if base.exists() else []
        completed = _run(["git", "ls-tree", "-r", "--name-only", self.rev, "--", prefix], cwd=self.repo)
        return sorted(line for line in completed.stdout.splitlines() if line)

    def read_bytes_many(self, paths: list[str]) -> dict[str, bytes]:
        if self.root is not None:
            return {path: (self.root / path).read_bytes() for path in paths}
        if not paths:
            return {}
        request = "".join(f"{self.rev}:{path}\n" for path in paths).encode()
        completed = subprocess.run(["git", "cat-file", "--batch"], cwd=self.repo, input=request, capture_output=True)
        if completed.returncode != 0:
            raise ToolError(f"git cat-file failed: {completed.stderr.decode(errors='replace').strip()}")
        blobs, data, offset = {}, completed.stdout, 0
        for path in paths:
            header_end = data.index(b"\n", offset)
            header = data[offset:header_end].decode()
            if header.endswith("missing"):
                raise ToolError(f"{path} does not exist at {self.rev}")
            size = int(header.split()[2])
            blobs[path] = data[header_end + 1:header_end + 1 + size]
            offset = header_end + 1 + size + 1
        return blobs


def _dispositions(manifest: dict[str, Any]) -> set[tuple[str, str]]:
    return {(d["path"], d["detector"]) for d in manifest.get("dispositions", [])}


def check_attributes(manifest: dict[str, Any], before: SourceReader, repo: Path = REPO_ROOT) -> dict[str, Any]:
    """Former file-level attributes versus new module declarations (S2 method)."""
    failures: list[str] = []
    review: list[dict[str, Any]] = []
    disposed = _dispositions(manifest)
    roots = {t["name"]: t["path"] for t in manifest["targets"]}
    root_declarations: dict[str, dict[str, list[str]] | None] = {}
    for name, path in roots.items():
        file = repo / path
        root_declarations[name] = module_declarations(file.read_text(encoding="utf-8")) if file.is_file() else None
        if root_declarations[name] is None:
            failures.append(f"{path}: consolidated root is missing")
        else:
            root_text = file.read_text(encoding="utf-8")
            for attribute in leading_inner_attributes(root_text):
                review.append({"path": path, "kind": "root-inner-attribute", "attribute": attribute})

    for module in manifest["modules"]:
        old_path, new_path = module["old_path"], module["new_path"]
        former = before.read(old_path)
        if former is None:
            failures.append(f"{old_path}: former source is not readable in the before state")
            continue
        new_file = repo / new_path
        if not new_file.is_file():
            failures.append(f"{new_path}: moved file is missing (from {old_path})")
            continue
        current = new_file.read_text(encoding="utf-8", errors="replace")
        declarations = root_declarations.get(module["target"])
        if declarations is None:
            continue
        if module["module"] not in declarations:
            failures.append(f"{roots[module['target']]}: does not declare `mod {module['module']};` (from {old_path})")
            continue
        declared = declarations[module["module"]]
        former_attributes = leading_inner_attributes(former)
        current_attributes = leading_inner_attributes(current)
        for attribute in former_attributes:
            name = _attribute_name(attribute)
            if name in CRATE_ROOT_ONLY:
                failures.append(f"{old_path}: crate-root-only attribute #![{attribute}] cannot survive as a module (R3: it forces a separate target or a root-level decision)")
            elif name == "cfg":
                if attribute not in declared:
                    failures.append(f"{roots[module['target']]}: `mod {module['module']};` lacks #[{attribute}] carried by {old_path} (acceptance 4: the condition belongs on the module declaration)")
                elif attribute in current_attributes:
                    review.append({"path": new_path, "kind": "inner-cfg-duplicated", "attribute": attribute})
            elif attribute not in declared and attribute not in current_attributes:
                failures.append(f"{new_path}: former file attribute #![{attribute}] from {old_path} is on neither the module file nor its declaration")
        for attribute in current_attributes:
            if _attribute_name(attribute) in CRATE_ROOT_ONLY:
                failures.append(f"{new_path}: crate-root-only attribute #![{attribute}] survived modularization; rustc only warns and drops it")

        if CRATE_PATH.search(_strip_comments(former)):
            if (new_path, "crate_path") not in disposed:
                failures.append(f"{old_path}: uses `crate::`, which re-points to the consolidated root once the file is a module; record a disposition")
        files = [new_file]
        if module["nested_root"]:
            files = sorted(new_file.parent.rglob("*.rs"))
        for file in files:
            rel = _rel(file, repo)
            text = file.read_text(encoding="utf-8", errors="replace")
            for detector, lines in _detector_hits(text, CRATE_GLOBAL_DETECTORS).items():
                if (rel, detector) not in disposed:
                    failures.append(f"{rel}:{lines[0]}: crate-global construct {detector} in a consolidated module; record a disposition or keep a separate target")
            identity = _detector_hits(text, IDENTITY_DETECTORS)
            if CURRENT_EXE.search(text) and LIST_ARG.search(text):
                identity["current_exe_list"] = [_line_of(text, CURRENT_EXE.search(text).start())]
            for detector, lines in identity.items():
                if (rel, detector) not in disposed:
                    failures.append(f"{rel}:{lines[0]}: identity-sensitive construct {detector} depends on the test path or binary (R9); record a disposition")
                else:
                    review.append({"path": rel, "kind": "identity-dispositioned", "detector": detector, "lines": lines})
            for detector, lines in _detector_hits(text, PATH_DETECTORS).items():
                review.append({"path": rel, "kind": "path-sensitive", "detector": detector, "lines": lines,
                               "old_dir": str(Path(old_path).parent.as_posix()), "new_dir": str(Path(new_path).parent.as_posix())})
    return {"kind": "consolidation-attribute-check", "version": 1, "package": manifest["package"], "failures": failures, "review": review}


def _strip_comments(text: str) -> str:
    """Rust source with comments blanked; string contents are left alone."""
    out = []
    index = 0
    while index < len(text):
        if text.startswith("//", index) or text.startswith("/*", index):
            end = _skip_trivia(text, index)
            out.append("\n" * text.count("\n", index, end))
            index = end
        elif text[index] == '"':
            end = _skip_string(text, index)
            out.append(text[index:end])
            index = end
        else:
            out.append(text[index])
            index += 1
    return "".join(out)


# ---------------------------------------------------------------------------
# check-snapshots
# ---------------------------------------------------------------------------


def _crate_name(target: str) -> str:
    return target.replace("-", "_")


def derive_snapshot_mapping(manifest: dict[str, Any], before: SourceReader) -> tuple[list[dict[str, str]], list[dict[str, str]], list[str]]:
    """(moved rows, unaffected rows, failures) by S4's rule, from tracked files.

    old: `<old root>/<rel>/snapshots/<old crate>__<rest>.snap`
    new: `<module dir>/<rel>/snapshots/<target crate>__<module>__<rest>.snap`
    """
    failures: list[str] = []
    crate_dir = manifest["crate_dir"]
    tests_prefix = f"{crate_dir}/tests/"
    by_crate = {_crate_name(m["old_target"]): m for m in manifest["modules"]}
    snapshots = [p for p in before.list_files(crate_dir) if p.endswith(".snap")]
    blobs = before.read_bytes_many(snapshots)
    moved, unaffected = [], []
    for path in snapshots:
        digest = hashlib.sha256(blobs[path]).hexdigest()
        if not path.startswith(tests_prefix):
            unaffected.append({"path": path, "sha256": digest})
            continue
        directory, _, filename = path.rpartition("/")
        if not directory.endswith("/snapshots") and directory != "snapshots":
            failures.append(f"{path}: not inside a `snapshots/` directory; cannot apply the mapping rule")
            continue
        source_dir = directory.removesuffix("/snapshots")
        name = filename.removesuffix(".snap")
        crate, separator, rest = name.partition("__")
        module = by_crate.get(crate)
        if module is None or not separator:
            failures.append(f"{path}: snapshot prefix {crate!r} names no old target in the manifest; it cannot be attributed")
            continue
        old_root = str(Path(module["old_path"]).parent.as_posix())
        if not (source_dir == old_root or source_dir.startswith(old_root + "/")):
            failures.append(f"{path}: snapshot directory is outside its target's source tree {old_root}")
            continue
        relative = source_dir[len(old_root):].lstrip("/")
        new_dir = str(Path(module["new_path"]).parent.as_posix())
        new_source_dir = f"{new_dir}/{relative}" if relative else new_dir
        new_path = f"{new_source_dir}/snapshots/{_crate_name(module['target'])}__{module['module']}__{rest}.snap"
        moved.append({"old": path, "new": new_path, "sha256": digest})
    return moved, unaffected, failures


def _sha256_file(path: Path) -> str | None:
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def check_snapshots(
    manifest: dict[str, Any],
    before: SourceReader,
    mapping: dict[str, Any],
    *,
    repo: Path = REPO_ROOT,
    captures: dict[CaptureKey, dict[str, Any]] | None = None,
) -> dict[str, Any]:
    failures: list[str] = []
    moved, unaffected, derive_failures = derive_snapshot_mapping(manifest, before)
    failures += derive_failures
    crate_dir = repo / manifest["crate_dir"]

    derived = {(row["old"], row["new"]): row for row in moved}
    table = {(row["old"], row["new"]): row for row in mapping.get("moved", [])}
    for key in sorted(derived.keys() - table.keys()):
        failures.append(f"{key[0]}: no mapping row (the rule places it at {key[1]})")
    for key in sorted(table.keys() - derived.keys()):
        failures.append(f"mapping row {key[0]} → {key[1]} disagrees with the rule or names an untracked snapshot")
    for key in sorted(derived.keys() & table.keys()):
        row = table[key]
        if row.get("sha256") != derived[key]["sha256"]:
            failures.append(f"{key[0]}: mapping row hash {row.get('sha256')} is not the before content hash {derived[key]['sha256']}")
        now = _sha256_file(repo / key[1])
        if now is None:
            failures.append(f"{key[1]}: mapped snapshot is missing after the move")
        elif now != derived[key]["sha256"]:
            failures.append(f"{key[1]}: content changed during the move (snapshots move byte for byte; regeneration is forbidden)")
        if key[0] != key[1] and (repo / key[0]).exists():
            failures.append(f"{key[0]}: old snapshot still exists after the move")
    for row in unaffected:
        now = _sha256_file(repo / row["path"])
        if now != row["sha256"]:
            failures.append(f"{row['path']}: unmoved snapshot changed or disappeared")
    table_unaffected = {row["path"] for row in mapping.get("unaffected", [])}
    if table_unaffected != {row["path"] for row in unaffected}:
        failures.append(f"the mapping's unaffected list differs from the tracked unmoved snapshots ({len(table_unaffected)} vs {len(unaffected)})")

    for pending in sorted(p for p in crate_dir.rglob("*") if p.name.endswith((".snap.new", ".pending-snap"))):
        failures.append(f"{_rel(pending, repo)}: pending snapshot file exists")
    for source in sorted((crate_dir / "tests").rglob("*.rs")):
        text = source.read_text(encoding="utf-8", errors="replace")
        for match in INSTA_PATH_SETTINGS.finditer(text):
            failures.append(f"{_rel(source, repo)}:{_line_of(text, match.start())}: Insta setting {match.group(1)} changes snapshot paths; the mechanical rule does not hold for this module")
    for config in (crate_dir / "insta.yaml", crate_dir / ".config" / "insta.yaml", repo / ".config" / "insta.yaml", repo / "insta.yaml"):
        if config.is_file():
            failures.append(f"{_rel(config, repo)}: an Insta configuration file exists; review it against the mapping rule")
    for recipe in [repo / "justfile", *sorted((repo / "just").glob("*.just")), *sorted(repo.glob("*/justfile")), *sorted((repo / ".github").rglob("*.yml"))]:
        if recipe.is_file() and "INSTA_REQUIRE_FULL_MATCH" in recipe.read_text(encoding="utf-8", errors="replace"):
            failures.append(f"{_rel(recipe, repo)}: sets INSTA_REQUIRE_FULL_MATCH, under which byte-identical moved snapshots fail (S4)")

    readers = snapshot_readers(manifest, before, moved, captures) if captures else {}
    return {
        "kind": "consolidation-snapshot-check",
        "version": 1,
        "package": manifest["package"],
        "moved": len(moved),
        "unaffected": len(unaffected),
        "readers": readers,
        "failures": failures,
    }


def snapshot_readers(
    manifest: dict[str, Any],
    before: SourceReader,
    moved: list[dict[str, str]],
    captures: dict[CaptureKey, dict[str, Any]],
) -> dict[str, Any]:
    """Per moved snapshot: whether a running test reads it.

    Each assertion's snapshot name is derived from its first argument (a
    string literal, a `format!` pattern, or the function name for an unnamed
    assertion); a snapshot is attributed to the most specific matching
    assertion. A snapshot read only by `#[ignore]`d tests, or with no
    attributable reader, is proven by the rule and its byte hash, not by a
    passing run, and the result says so for exactly those files.
    """
    crate_dir = manifest["crate_dir"]
    rows_by_crate: dict[str, list[dict[str, str]]] = defaultdict(list)
    modules = {_crate_name(m["old_target"]): m for m in manifest["modules"]}
    for row in moved:
        rows_by_crate[row["old"].rpartition("/")[2].partition("__")[0]].append(row)
    listed: dict[str, dict[str, bool]] = defaultdict(dict)  # binary id → test leaf → ignored everywhere
    for (_h, package, _fs), capture in captures.items():
        if package != manifest["package"]:
            continue
        for binary_id, suite in capture["suites"].items():
            for test, record in suite["tests"].items():
                leaf = test.rpartition("::")[2]
                listed[binary_id][leaf] = listed[binary_id].get(leaf, True) and record["ignored"]
    result = {}
    for crate, rows in sorted(rows_by_crate.items()):
        module = modules[crate]
        directories = sorted({row["old"].rpartition("/")[0].removesuffix("/snapshots") for row in rows})
        files = []
        for directory in directories:
            if directory == f"{crate_dir}/tests":
                files.append(module["old_path"])
            else:
                files += [p for p in before.list_files(directory) if p.endswith(".rs") and p.rpartition("/")[0] == directory]
        functions: dict[str, str] = {}
        patterns: list[tuple[int, str, re.Pattern[str] | None]] = []
        tests_here = listed.get(module["old_binary_id"], {})
        for file in files:
            text = before.read(file) or ""
            blank = _blank_literals(text)
            spans = _function_spans(blank)
            for assertion in SNAPSHOT_ASSERT.finditer(blank):
                enclosing = [name for start, end, name in spans if start < assertion.start() < end]
                # Innermost first: a helper nested inside a test is not the reader.
                owner = next((name for name in reversed(enclosing) if name in tests_here), enclosing[-1] if enclosing else None)
                if owner is None:
                    continue
                state = tests_here.get(owner)
                functions[owner] = "not-a-listed-test" if state is None else ("ignored" if state else "running")
                named = _snapshot_name_pattern(text, assertion.end(), owner)
                if named is not None:
                    patterns.append((named[0], owner, named[1]))
        attributed: dict[str, list[str]] = defaultdict(list)
        for row in rows:
            name = row["old"].rpartition("/")[2].removesuffix(".snap").partition("__")[2]
            matches = [(specificity, owner) for specificity, owner, pattern in patterns if pattern is not None and pattern.fullmatch(name)]
            best = max((s for s, _ in matches), default=None)
            owners = {owner for s, owner in matches if s == best}
            states = {functions[owner] for owner in owners}
            state = states.pop() if len(states) == 1 else "unresolved"
            attributed["unresolved" if state == "not-a-listed-test" else state].append(row["old"])
        silent = sorted(attributed.get("ignored", [])) + sorted(attributed.get("unresolved", []))
        if not silent:
            statement = "every moved snapshot is read by a running test"
        else:
            statement = (f"{len(attributed.get('ignored', []))} of {len(rows)} moved snapshots are read only by #[ignore]d tests and "
                         f"{len(attributed.get('unresolved', []))} have no attributable reader; those are proven by rule and byte hash only, not by a passing run")
        result[module["old_target"]] = {
            "asserting_functions": functions,
            "snapshots": {state: len(paths) for state, paths in sorted(attributed.items())},
            "read_by_no_running_test": sorted(attributed.get("ignored", [])),
            "reader_unresolved": sorted(attributed.get("unresolved", [])),
            "statement": statement,
        }
    return result


def _blank_literals(text: str) -> str:
    """Same-length source with comment and string/char literal contents blanked."""
    out = []
    index = 0
    while index < len(text):
        char = text[index]
        if text.startswith("//", index) or text.startswith("/*", index):
            end = _skip_trivia(text, index)
        elif char == '"' or (char in "rb" and re.match(r'b?r#*"', text[index:])) or (char == "'" and re.match(r"'(\\u\{[0-9a-fA-F]+\}|\\.|[^\\'])'", text[index:])):
            end = _skip_string(text, index)
        else:
            out.append(char)
            index += 1
            continue
        out.append("".join(c if c == "\n" else " " for c in text[index:end]))
        index = end
    return "".join(out)


def _function_spans(blank: str) -> list[tuple[int, int, str]]:
    """(start, end, name) of every `fn` with a body, outermost first by start."""
    spans = []
    for match in FN_DEF.finditer(blank):
        brace, semicolon = blank.find("{", match.end()), blank.find(";", match.end())
        if brace < 0 or (0 <= semicolon < brace):
            continue
        spans.append((match.start(), _balanced(blank, brace, "{", "}"), match.group(1)))
    return spans


def _split_top_level(text: str) -> list[str]:
    """Macro arguments split on commas outside brackets and strings."""
    parts, depth, start, index = [], 0, 0, 0
    while index < len(text):
        char = text[index]
        if char == '"' or (char in "rb" and re.match(r'b?r#*"', text[index:])):
            index = _skip_string(text, index)
            continue
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        elif char == "," and depth == 0:
            parts.append(text[start:index].strip())
            start = index + 1
        index += 1
    tail = text[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def _snapshot_name_pattern(text: str, after_bang: int, owner: str) -> tuple[int, re.Pattern[str] | None] | None:
    """(specificity, name regex) for the assertion whose `!` ends at `after_bang`.

    `None` for an inline (`@"…"`) snapshot, which has no file. A pattern of
    `None` means the name is computed at runtime from something other than a
    literal or `format!` string.
    """
    open_paren = _skip_trivia(text, after_bang)
    if open_paren >= len(text) or text[open_paren] not in "([{":
        return (0, None)
    close = {"(": ")", "[": "]", "{": "}"}[text[open_paren]]
    arguments = _split_top_level(text[open_paren + 1:_balanced(text, open_paren, text[open_paren], close) - 1])
    if any(argument.startswith("@") for argument in arguments):
        return None
    if len(arguments) <= 1:
        return (len(owner), re.compile(rf"(?:.+__)?{re.escape(owner)}(?:-\d+)?"))
    first = arguments[0].lstrip("&").strip()
    literal = re.fullmatch(r'"((?:[^"\\]|\\.)*)"', first)
    if literal:
        return (len(literal.group(1)), re.compile(rf"(?:.+__)?{re.escape(literal.group(1))}"))
    formatted = re.match(r'format!\s*\(\s*"((?:[^"\\]|\\.)*)"', first)
    if formatted:
        pieces = re.split(r"(?<!\{)\{[^{}]*\}(?!\})", formatted.group(1))
        pieces = [piece.replace("{{", "{").replace("}}", "}") for piece in pieces]
        return (sum(len(p) for p in pieces), re.compile("(?:.+__)?" + ".+".join(re.escape(p) for p in pieces)))
    return (0, None)


# ---------------------------------------------------------------------------
# Shared helpers and CLI
# ---------------------------------------------------------------------------


def _run(command: list[str], *, cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    try:
        completed = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace")
    except FileNotFoundError as error:
        raise ToolError(f"{command[0]} is not installed: {error}") from error
    if completed.returncode != 0:
        tail = "\n".join(completed.stderr.strip().splitlines()[-15:])
        raise ToolError(f"`{' '.join(command)}` exited {completed.returncode}:\n{tail}")
    return completed


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise ToolError(f"cannot read {path}: {error}") from error


def _rel(path: Path, repo: Path) -> str:
    try:
        return Path(path).resolve().relative_to(repo.resolve()).as_posix()
    except ValueError:
        return Path(path).as_posix()


def _load_json(path: Path) -> Any:
    try:
        return json.loads(_read_maybe_gzip(Path(path)))
    except json.JSONDecodeError as error:
        raise ToolError(f"{path}: invalid JSON: {error}") from error


def _write_json(path: Path | None, document: Any) -> None:
    text = json.dumps(document, indent=2, sort_keys=True) + "\n"
    if path is None:
        sys.stdout.write(text)
    else:
        Path(path).write_text(text, encoding="utf-8")


def _finish(failures: list[str]) -> int:
    for failure in failures:
        print(f"FAIL: {failure}", file=sys.stderr)
    return 1 if failures else 0


def _before_reader(args: argparse.Namespace) -> SourceReader:
    if args.before_root:
        return SourceReader(root=Path(args.before_root))
    return SourceReader(rev=args.before_rev or "HEAD")


def parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)

    inventory = sub.add_parser("inventory", help="merge cargo metadata and manifests into the target table")
    inventory.add_argument("--package", action="append", help="package to include (repeatable; default: the four in scope)")
    inventory.add_argument("--listings", action="append", default=[], help="capture documents or raw listings for tier counts")
    inventory.add_argument("--out", type=Path, help="inventory JSON (default: stdout)")
    inventory.add_argument("--markdown", type=Path, help="also write a human summary")

    plan = sub.add_parser("plan", help="emit the migration manifest for one package")
    plan.add_argument("--package", required=True)
    plan.add_argument("--listings", action="append", required=True, help="before captures (every host available)")
    plan.add_argument("--inventory", type=Path, help="inventory JSON (default: generated from the live tree)")
    plan.add_argument("--out", type=Path, help="manifest JSON (default: stdout)")

    capture = sub.add_parser("capture", help="list every selector for every feature set with nextest")
    capture.add_argument("--package", action="append", help="package to capture (repeatable; default: the four in scope)")
    capture.add_argument("--feature-set", action="append", help="comma-separated features, or 'none' (repeatable; default: the package's known sets)")
    capture.add_argument("--out", type=Path, required=True)
    capture.add_argument("--raw-dir", type=Path, help="also keep every raw listing here")

    compare = sub.add_parser("compare", help="four-way identity comparison of before and after captures")
    compare.add_argument("--before", action="append", required=True)
    compare.add_argument("--after", action="append", required=True)
    compare.add_argument("--manifest", action="append", default=[], help="migration manifest (one per migrated package)")
    compare.add_argument("--package", action="append")
    compare.add_argument("--common-selectors", action="store_true", help="compare only selectors both sides captured (reported as notes)")
    compare.add_argument("--require-identical-digests", action="store_true", help="also require every raw listing digest to match (no-op proofs)")
    compare.add_argument("--out", type=Path)
    compare.add_argument("--markdown", type=Path)

    for name, helptext in (("check-attributes", "compare former crate attributes with new module declarations"),
                           ("check-snapshots", "validate the old→new snapshot mapping")):
        checker = sub.add_parser(name, help=helptext)
        checker.add_argument("--manifest", type=Path, required=True)
        source = checker.add_mutually_exclusive_group()
        source.add_argument("--before-rev", help="git revision holding the before state (default: HEAD)")
        source.add_argument("--before-root", help="directory holding the before state")
        checker.add_argument("--out", type=Path)
        if name == "check-snapshots":
            mode = checker.add_mutually_exclusive_group(required=True)
            mode.add_argument("--mapping", type=Path, help="committed mapping table to validate")
            mode.add_argument("--emit-mapping", type=Path, help="write the rule-derived mapping table")
            checker.add_argument("--listings", action="append", default=[], help="before captures, to state which snapshots a running test reads")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        if args.command == "inventory":
            captures = load_captures(Path(p) for p in args.listings) if args.listings else {}
            inventory, failures = build_inventory(cargo_metadata(), REPO_ROOT, packages=args.package or PACKAGES, captures=captures)
            _write_json(args.out, inventory)
            if args.markdown:
                args.markdown.write_text(render_inventory_markdown(inventory) + "\n", encoding="utf-8")
            return _finish(failures)
        if args.command == "plan":
            captures = load_captures(Path(p) for p in args.listings)
            if args.inventory:
                inventory, failures = _load_json(args.inventory), []
            else:
                inventory, failures = build_inventory(cargo_metadata(), REPO_ROOT, packages=[args.package])
            manifest_path = REPO_ROOT / inventory["packages"][args.package]["manifest"]
            result = plan_package(args.package, inventory, captures, selector_filters(args.package, manifest_path))
            _write_json(args.out, result.manifest)
            return _finish(failures + result.failures)
        if args.command == "capture":
            feature_sets = None
            if args.feature_set:
                feature_sets = [() if f == "none" else tuple(f.split(",")) for f in args.feature_set]
            for package in args.package or PACKAGES:
                capture_package(package, args.out, feature_sets=feature_sets, raw_dir=args.raw_dir)
            return 0
        if args.command == "compare":
            manifests = {}
            for path in args.manifest:
                document = _load_json(Path(path))
                manifests[document["package"]] = document
            report = compare_captures(
                load_captures(Path(p) for p in args.before),
                load_captures(Path(p) for p in args.after),
                manifests,
                packages=args.package,
                common_selectors=args.common_selectors,
                require_identical_digests=args.require_identical_digests,
            )
            if args.out:
                _write_json(args.out, report)
            if args.markdown:
                args.markdown.write_text(render_comparison_markdown(report), encoding="utf-8")
            print(f"compare: {report['verdict']} ({len(report['failures'])} failures, {len(report['notes'])} notes)", file=sys.stderr)
            return _finish(report["failures"])
        if args.command == "check-attributes":
            report = check_attributes(_load_json(args.manifest), _before_reader(args))
            _write_json(args.out, report)
            return _finish(report["failures"])
        if args.command == "check-snapshots":
            manifest = _load_json(args.manifest)
            before = _before_reader(args)
            if args.emit_mapping:
                moved, unaffected, failures = derive_snapshot_mapping(manifest, before)
                _write_json(args.emit_mapping, {"kind": "consolidation-snapshot-mapping", "version": 1, "package": manifest["package"], "moved": moved, "unaffected": unaffected})
                return _finish(failures)
            captures = load_captures(Path(p) for p in args.listings) if args.listings else None
            report = check_snapshots(manifest, before, _load_json(args.mapping), captures=captures)
            _write_json(args.out, report)
            return _finish(report["failures"])
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    raise AssertionError(f"unhandled command {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
