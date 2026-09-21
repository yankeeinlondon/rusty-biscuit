#!/usr/bin/env python3
"""Prove one producer cell ran the work the plan scheduled, then certify it.

A green JUnit report says what ran. It cannot say what *should* have run: a
drifted filter expression, an unprovisioned backend, or a lost report all leave
the same silence behind, and `--no-tests=pass` turns the widest of those
failures into an exit code of zero. This tool closes that by comparing the
producer's own expected-test listing against its own staged reports, as
identities rather than counts, and writing the cell's completion record only
when every check holds (ruling R10).

Python 3 stdlib only, because an archive consumer has no compiler and the WSL2
guest installs a thin apt set (ruling R2).

## Examples

```bash
python3 scripts/ci/completion.py \
    --plan ci-artifacts/ci-resolved-plan/resolved-plan.json \
    --cell claudine/ubuntu-latest/L1 \
    --expected-manifest target/nextest/ci-reports/expected-L1.json \
    --artifacts ci-artifacts \
    --companions "$RUNNER_TEMP/companions.json" \
    --backend-proofs target/nextest/ci-reports/backend-proofs.json \
    --target x86_64-unknown-linux-gnu \
    --run "$GITHUB_RUN_ID" --attempt "$GITHUB_RUN_ATTEMPT" \
    --out "$RUNNER_TEMP/completion.json"
```

## Exit status

`0` and the record at `--out` exactly when the cell is proven. `1` when the
cell did not prove itself, `2` when this tool could not read its own inputs —
both fail the producer, and the split is so an unreadable input is never
reported as an invented test failure. A nonzero exit names every refusal on
stderr and leaves **no** record at `--out`, including one an earlier validation
of the same path wrote, so `complete: true` cannot exist without the evidence
behind it. Diagnostics the job publishes on the failure path (the status
artifact, the JUnit reports) are untouched.

## Notes

The two rules that are easy to get backwards:

* An **observed** `<skipped/>` may be excused by an unexpired approval in the
  plan's `skip_policy`. An **absent** test may not, by anything. This is the
  specification's one intentional tightening: a skip is a decision somebody
  recorded, and silence is a lost test.
* A test compiled out by `cfg` on this target is in none of the manifest's
  three identity sets — it does not exist here — which is why a manifest listed
  on another target is refused outright rather than diffed.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from datetime import date
from pathlib import Path
from typing import Any
from xml.etree import ElementTree

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import schema  # noqa: E402  (needs the path insert above)


#: Predicates a canonical selection may use. `_tier_filter` builds every tier
#: expression from `test(...)` alone and archive mode wraps it in
#: `package(...)`; anything else — `binary(…)`, `kind(…)`, `all()`, a bare
#: test name — narrows the run to something the plan did not schedule.
CANONICAL_PREDICATES = ("package", "test")

_PREDICATE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)\s*\(")

#: A tier marker as `_tier_filter` anchors it: `/(^|::)<marker>/`. The markers
#: live in `schema.TIER_MARKERS` and which of them each gate selects or
#: excludes in `schema.CANONICAL_SELECTION`; the expressions stay in the
#: justfile, so [`canonical_selection_problems`] rebuilds the gate's shape
#: from that data and never compares against a copy of the strings.
_TIER_MARKER = re.compile(
    r"\A/\(\^\|::\)(" + "|".join(schema.TIER_MARKERS) + r")/\Z"
)

#: How `_stage_junit` names a cell's report directory under the artifacts root.
JUNIT_DIRECTORY = "junit-{package}-{gate}-{environment}"


class Refused(Exception):
    """The cell is not proven. Carries the coded reasons, one per line.

    `infrastructure` separates "this tool could not read its own inputs" from
    "this cell did not prove itself". Both fail the producer, and neither is a
    test result — but only the second is a coverage verdict, and an unreadable
    input must never be reported as an invented test failure.
    """

    def __init__(self, problems: list[str], infrastructure: bool = False) -> None:
        super().__init__("\n".join(problems))
        self.problems = problems
        self.infrastructure = infrastructure


def refuse(code: str, message: str, infrastructure: bool = False) -> Refused:
    return Refused([f"{code}: {message}"], infrastructure)


# ---------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------


def read_json(path: Path, code: str, what: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise refuse(code, f"{what} is absent at {path}", infrastructure=True) from error
    except (OSError, json.JSONDecodeError) as error:
        raise refuse(
            code, f"{what} at {path} could not be read: {error}", infrastructure=True
        ) from error


def split_cell(value: str) -> tuple[str, str, str]:
    parts = value.split("/")
    if len(parts) != 3 or not all(parts):
        raise refuse(
            "completion-cell-unknown",
            f"--cell {value!r} is not '<package>/<environment>/<gate>'",
        )
    return parts[0], parts[1], parts[2]


def cell_of(plan: Any, package: str, environment: str, gate: str) -> dict[str, Any]:
    """The one executing cell `{package, environment, gate}` names in the plan."""
    if not isinstance(plan, dict) or not isinstance(plan.get("cells"), list):
        raise refuse("completion-plan-unreadable", "the plan carries no cell list")
    matches = [
        cell
        for cell in plan["cells"]
        if isinstance(cell, dict)
        and cell.get("package") == package
        and cell.get("environment") == environment
        and cell.get("gate") == gate
    ]
    if not matches:
        raise refuse(
            "completion-cell-unknown",
            f"the plan carries no cell {package}/{environment}/{gate}",
        )
    if len(matches) > 1:
        raise refuse(
            "completion-cell-unknown",
            f"the plan carries {len(matches)} cells {package}/{environment}/{gate}; "
            "a cell key names one unit of work",
        )
    cell = matches[0]
    if cell.get("execution") != "execute":
        raise refuse(
            "completion-cell-not-executing",
            f"cell {package}/{environment}/{gate} is {cell.get('execution')!r} in the "
            "plan; only an executing cell has work to prove",
        )
    return cell


def package_of(plan: dict[str, Any], package: str) -> dict[str, Any]:
    for record in plan.get("packages", []):
        if isinstance(record, dict) and record.get("package") == package:
            return record
    raise refuse(
        "completion-plan-unreadable", f"the plan carries no package record for {package}"
    )


# ---------------------------------------------------------------------------
# The expected-test manifest
# ---------------------------------------------------------------------------


def selection_problems(selection: Any, cell: dict[str, Any]) -> list[str]:
    """Every reason the listing's recorded selection is not the cell's.

    The narrowing an identity comparison cannot see on its own: an ad hoc
    `-E 'test(one_name)'` — or a drifted `_tier_filter` that listed and ran
    another tier — shrinks the expected set and the observed set together, so
    both agree and both are wrong. What catches it is that the manifest
    recorded *what it selected with*, and the cell's gate fixes which
    canonical selection that must be.
    """
    problems = [
        f"completion-manifest-selection: the listing's selection is missing "
        f"'{name}'"
        for name, required in schema.EXPECTED_SELECTION_FIELDS.items()
        if required and (not isinstance(selection, dict) or name not in selection)
    ]
    if problems or not isinstance(selection, dict):
        return problems or [
            "completion-manifest-selection: the listing's selection must be an object"
        ]

    expression = selection["filter"]
    if not isinstance(expression, str) or not expression.strip():
        problems.append(
            "completion-manifest-selection: the listing recorded no filter "
            "expression, so nothing says the tier's own selection was applied"
        )
    else:
        foreign = [
            f"completion-manifest-selection: the listing selected with "
            f"'{name}(…)', which no tier expression uses; the run was "
            f"narrowed to something the plan did not schedule"
            for name, _argument in predicates(expression)
            if name not in CANONICAL_PREDICATES
        ]
        # A foreign predicate is conclusive on its own; the shape check would
        # only restate it.
        problems += foreign or canonical_selection_problems(
            expression, cell["gate"], cell.get("package")
        )

    profile = cell.get("profile")
    if profile is not None and selection["profile"] != profile:
        problems.append(
            f"completion-manifest-selection: the listing used profile "
            f"{selection['profile']!r}, the plan's cell {profile!r}"
        )

    # S2's first condition: `--run-ignored` flips ignored tests to `matches`,
    # so the manifest's three identity sets would no longer mean what they say.
    if "--run-ignored" in str(selection["test_args"]):
        problems.append(
            "completion-manifest-selection: the listing passed --run-ignored, "
            "which moves ignored tests into the expected set"
        )
    return problems


def predicates(expression: str) -> list[tuple[str, str]]:
    """Every `name(argument)` in a filterset expression, arguments verbatim.

    Depth counting rather than a regex for the argument: a tier marker's own
    regex body carries balanced parentheses (`/(^|::)level2_/`).
    """
    found: list[tuple[str, str]] = []
    for match in _PREDICATE.finditer(expression):
        close = _closing(expression, match.end() - 1)
        if close is not None:
            found.append((match.group(1), expression[match.end() : close]))
    return found


def canonical_selection_problems(
    expression: str, tier: str, package: str | None
) -> list[str]:
    """Every reason `expression` is not the canonical selection for `tier`.

    The check the marker vocabulary alone cannot make: `test(/(^|::)level2_/)`
    is built from a tier marker and is still the wrong selection for an L1
    cell. The expected shape comes from `schema.CANONICAL_SELECTION` and the
    gate, never from the justfile, so a `_tier_filter` that drifted cannot
    satisfy a validator that drifted with it.

    Accepted, whitespace-insensitive: an optional `package(<package>) & ( … )`
    wrapper (archive mode), around either exactly `test(/(^|::)<marker>/)` for a
    positive tier or `!(test(…) + test(…) + …)` for an L1-shaped one whose
    negated markers cover every `must` marker and nothing outside `must ∪ may`.
    """
    contract = schema.CANONICAL_SELECTION
    selects = contract["selects"]
    excludes = contract["excludes"]
    if tier in selects:
        expected = f"test(/(^|::){selects[tier]}/)"
    elif tier in excludes["tiers"]:
        expected = (
            f"!(test(…) + …) negating every marker in {excludes['must']} and no "
            f"marker outside those plus {excludes['may']}"
        )
    else:
        return [
            f"completion-manifest-selection: no canonical selection is defined "
            f"for gate {tier!r}, so the listing's filter cannot be checked"
        ]

    def refuse(why: str) -> list[str]:
        return [
            f"completion-manifest-selection: the listing selected with "
            f"{expression.strip()!r}, which is not the canonical {tier} selection "
            f"({why}); {tier} expects {expected}, optionally inside "
            f"'package(<package>) & (…)'"
        ]

    body = expression.strip()
    scope = _term(body)
    if scope is not None and scope[0] == "package":
        name, argument, rest = scope
        if argument.strip() != (package or ""):
            return refuse(f"it is scoped to package {argument.strip()!r}, the cell's is {package!r}")
        rest = rest.strip()
        if not rest.startswith("&"):
            return refuse("a package scope must be joined to the tier expression with '&'")
        rest = rest[1:].strip()
        close = _closing(rest, 0) if rest.startswith("(") else None
        if close != len(rest) - 1:
            return refuse("the tier expression after the package scope must be parenthesized")
        body = rest[1:close].strip()

    markers: list[str]
    if body.startswith("!"):
        if tier not in excludes["tiers"]:
            return refuse("a positive tier does not negate")
        inner = body[1:].strip()
        close = _closing(inner, 0) if inner.startswith("(") else None
        if close != len(inner) - 1:
            return refuse("the negated union must be one parenthesized group")
        terms = _split_union(inner[1:close])
        if not terms:
            return refuse("the negated group is empty")
        markers = []
        for text in terms:
            marker = _marker(text)
            if marker is None:
                return refuse(f"{text.strip()!r} is not test(/(^|::)<marker>/)")
            markers.append(marker)
        missing = [marker for marker in excludes["must"] if marker not in markers]
        allowed = set(excludes["must"]) | set(excludes["may"])
        extra = [marker for marker in markers if marker not in allowed]
        if missing:
            return refuse(f"it leaves {missing} inside the run")
        if extra:
            return refuse(f"it excludes {extra}, which no L1-shaped tier may")
        return []

    if tier not in selects:
        return refuse("an L1-shaped tier negates the other tiers' markers")
    marker = _marker(body)
    if marker is None:
        return refuse("it is not exactly one test(/(^|::)<marker>/) predicate")
    if marker != selects[tier]:
        return refuse(f"it selects the {marker!r} tier")
    return []


def _closing(text: str, open_index: int) -> int | None:
    """Index of the parenthesis closing the one at `open_index`, or None."""
    depth = 0
    for index in range(open_index, len(text)):
        if text[index] == "(":
            depth += 1
        elif text[index] == ")":
            depth -= 1
            if depth == 0:
                return index
    return None


def _term(text: str) -> tuple[str, str, str] | None:
    """`(name, argument, rest)` when `text` opens with a predicate call."""
    head = _PREDICATE.match(text)
    if head is None:
        return None
    close = _closing(text, head.end() - 1)
    if close is None:
        return None
    return head.group(1), text[head.end() : close], text[close + 1 :]


def _marker(text: str) -> str | None:
    """The tier marker when `text` is exactly one `test(/(^|::)<marker>/)`."""
    term = _term(text.strip())
    if term is None or term[0] != "test" or term[2].strip():
        return None
    match = _TIER_MARKER.match(term[1].strip())
    return match.group(1) if match else None


def _split_union(text: str) -> list[str]:
    """The operands of a top-level `+` union, parentheses respected."""
    terms: list[str] = []
    depth = 0
    start = 0
    for index, char in enumerate(text):
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif char == "+" and depth == 0:
            terms.append(text[start:index])
            start = index + 1
    terms.append(text[start:])
    return [term for term in terms if term.strip()]


def manifest_problems(
    manifest: Any,
    cell: dict[str, Any],
    package: dict[str, Any],
    target: str,
) -> list[str]:
    """Every reason the expected-test manifest cannot be compared to a report."""
    if not isinstance(manifest, dict):
        return ["completion-manifest-schema: the expected manifest must be an object"]

    version = manifest.get("schema_version")
    if version != schema.EXPECTED_MANIFEST_SCHEMA_VERSION:
        return [
            f"completion-manifest-schema: the expected manifest is schema_version "
            f"{version!r}; this tool reads "
            f"{schema.EXPECTED_MANIFEST_SCHEMA_VERSION}, which records the ignored "
            f"and excluded identities an earlier version dropped"
        ]

    problems = [
        f"completion-manifest-provenance: the expected manifest is missing "
        f"'{name}'"
        for name, required in schema.EXPECTED_MANIFEST_FIELDS.items()
        if required and name not in manifest
    ]
    problems += [
        f"completion-manifest-provenance: the expected manifest has unknown field "
        f"'{name}'"
        for name in sorted(manifest)
        if name not in schema.EXPECTED_MANIFEST_FIELDS
    ]
    if problems:
        return problems

    if manifest["target"] != target:
        problems.append(
            f"completion-manifest-target: the expected manifest was listed on target "
            f"{manifest['target']!r}, this cell executes {target!r}; a cfg-compiled "
            f"test set differs between targets, so the two are not comparable"
        )
    if manifest["environment"] != cell["environment"]:
        problems.append(
            f"completion-manifest-provenance: the expected manifest names environment "
            f"{manifest['environment']!r}, the cell {cell['environment']!r}"
        )
    if manifest["tier"] != cell["gate"]:
        problems.append(
            f"completion-manifest-provenance: the expected manifest names tier "
            f"{manifest['tier']!r}, the cell's gate is {cell['gate']!r}; an L1 report "
            f"is never compared against another tier in a shared archive"
        )
    if not str(manifest["nextest_version"]).strip():
        problems.append(
            "completion-manifest-provenance: the expected manifest records no "
            "nextest_version, so a cross-host listing mismatch could not be seen"
        )

    problems += selection_problems(manifest["selection"], cell)
    problems += declared_problems(manifest, cell)

    packages = manifest.get("packages")
    if not isinstance(packages, dict) or cell["package"] not in packages:
        problems.append(
            f"completion-manifest-provenance: the expected manifest lists no "
            f"identities for {cell['package']}"
        )
    else:
        listing = packages[cell["package"]]
        problems += [
            f"completion-manifest-provenance: the expected manifest's "
            f"{cell['package']} listing is missing '{name}'"
            for name, required in schema.EXPECTED_PACKAGE_FIELDS.items()
            if required and (not isinstance(listing, dict) or name not in listing)
        ]
    return problems


def declared_problems(manifest: dict[str, Any], cell: dict[str, Any]) -> list[str]:
    """Every reason the producer's declared work differs from the plan's.

    Optional on the manifest, and cross-checked rather than trusted: the plan
    is the source of truth, so a disagreement means the job was provisioned for
    different work than the plan scheduled — which is a defect wherever it was
    introduced, not a value to prefer one side of.
    """
    problems = []
    if "backends" in manifest:
        declared = sorted(set(manifest["backends"]))
        planned = sorted(set(required_backends(cell)))
        if declared != planned:
            problems.append(
                f"completion-manifest-selection: the producer required backends "
                f"{declared}, the plan requires {planned} of this cell"
            )
    if "companion_suites" in manifest:
        declared = sorted(set(manifest["companion_suites"]))
        planned = sorted(entry["name"] for entry in cell.get("companions", []))
        if declared != planned:
            problems.append(
                f"completion-manifest-selection: the producer ran companion suites "
                f"{declared}, the plan attaches {planned}"
            )
    return problems


def required_backends(cell: dict[str, Any]) -> list[str]:
    """The terminal backends this cell must prove drove a test.

    Read off the cell, not the package: the planner attaches the subset of
    `l2_backends` the cell's environment can host, which is exactly what the
    producer set `BISCUIT_TEST_REQUIRED_BACKENDS` to. The package-wide list
    names GUI backends no runner hosts, and demanding those refused every
    mixed-backend cell. A backend that is never provisioned skips its suite,
    and nextest prints PASS in about 0.02 seconds — indistinguishable from
    evidence unless the proof is read.
    """
    if cell["gate"] != "L2":
        return []
    backends = cell.get("backends")
    if (
        not isinstance(backends, list)
        or not backends
        or not all(isinstance(backend, str) and backend for backend in backends)
    ):
        raise refuse(
            "completion-plan-unreadable",
            f"the executing L2 cell {cell['package']}/{cell['environment']}/L2 "
            "names no required backend; a plan of schema version "
            f"{schema.RESOLVED_PLAN_SCHEMA_VERSION} attaches the hostable subset of "
            "l2_backends to every executing L2 cell",
            infrastructure=True,
        )
    return list(backends)


# ---------------------------------------------------------------------------
# The staged reports
# ---------------------------------------------------------------------------


def staging_root(artifacts: Path, cell: dict[str, Any]) -> Path:
    """Where this cell's staged reports are, under the downloaded artifacts.

    Two spellings because both are real: a producer validates its own staging
    directory in place, and an artifact consumer sees it as
    `junit-<package>-<gate>-<environment>/`.
    """
    if (artifacts / "manifest.jsonl").is_file():
        return artifacts
    named = artifacts / JUNIT_DIRECTORY.format(
        package=cell["package"], gate=cell["gate"], environment=cell["environment"]
    )
    if named.is_dir():
        return named
    raise refuse(
        "completion-report-missing",
        f"no JUnit staging tree for {cell['package']}/{cell['environment']}/"
        f"{cell['gate']} under {artifacts}: neither a manifest.jsonl here nor "
        f"{named.name}/",
    )


def staged_entries(stage: Path, cell: dict[str, Any]) -> list[dict[str, Any]]:
    """The manifest records this cell's gate staged, in the order it wrote them."""
    manifest = stage / "manifest.jsonl"
    if not manifest.is_file():
        raise refuse(
            "completion-report-missing",
            f"the staging tree at {stage} has no manifest.jsonl, so nothing says "
            "which reports this gate produced",
        )
    entries = []
    for number, line in enumerate(
        manifest.read_text(encoding="utf-8").splitlines(), start=1
    ):
        line = line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            raise refuse(
                "completion-report-malformed",
                f"{manifest} line {number} is malformed: {error}",
            ) from error
        if not isinstance(record, dict):
            raise refuse(
                "completion-report-malformed",
                f"{manifest} line {number} is malformed: not an object",
            )
        if record.get("tier") != cell["gate"] or record.get("package") != cell["package"]:
            continue
        environment = record.get("environment") or cell["environment"]
        if environment != cell["environment"]:
            continue
        entries.append(record)
    return entries


def report_outcomes(path: Path) -> dict[str, str]:
    """Every `<testsuite name>::<testcase name>` in one report, and its outcome.

    The identity convention is the binary's, not the module's: two binaries may
    carry the same test name, and collapsing them would let one satisfy the
    other's expectation.
    """
    try:
        root = ElementTree.parse(path).getroot()
    except ElementTree.ParseError as error:
        raise refuse(
            "completion-report-malformed",
            f"the report {path.name} is malformed and is not evidence: {error}",
        ) from error
    except OSError as error:
        raise refuse(
            "completion-report-missing", f"the report {path.name} could not be read: {error}"
        ) from error

    suites = [root] if root.tag == "testsuite" else list(root.iter("testsuite"))
    outcomes: dict[str, str] = {}
    for suite in suites:
        for case in suite.iter("testcase"):
            identity = f"{suite.get('name', '')}::{case.get('name', '')}"
            if case.find("failure") is not None or case.find("error") is not None:
                outcome = "fail"
            elif case.find("skipped") is not None:
                outcome = "skip"
            else:
                outcome = "pass"
            outcomes[identity] = outcome
    return outcomes


def observed_outcomes(
    stage: Path, entries: list[dict[str, Any]]
) -> tuple[dict[str, str], list[str]]:
    """The cell's final outcome per identity, and the reports it read.

    ## Notes

    Reports are grouped by their identity set, which is what "the same work"
    means here: a retry re-runs the same selection, so its report covers the
    same identities. Within a group the last staged report decides, and a
    failure in an earlier attempt is not hidden by that — the *last* attempt is
    the one the job ended on. A group whose reports also agree on every outcome
    retried nothing; it is the same result staged twice, which inflates the
    evidence, and a duplicate identity across *different* selections is the
    binary collision the identity convention exists to catch. Both refuse.
    """
    reports: list[tuple[str, dict[str, str]]] = []
    problems: list[str] = []
    for entry in entries:
        relative = str(entry.get("xml", ""))
        path = stage / relative
        if not relative or not path.is_file() or entry.get("report_present") is False:
            problems.append(
                f"completion-report-missing: the staging manifest records a report at "
                f"{relative or '<unnamed>'} that was never written; a gate with no "
                f"report proves nothing"
            )
            continue
        reports.append((relative, report_outcomes(path)))

    if problems:
        raise Refused(problems)

    groups: dict[frozenset[str], list[tuple[str, dict[str, str]]]] = {}
    for relative, outcomes in reports:
        groups.setdefault(frozenset(outcomes), []).append((relative, outcomes))

    final: dict[str, str] = {}
    for attempts in groups.values():
        if len(attempts) > 1 and all(
            outcomes == attempts[0][1] for _, outcomes in attempts[1:]
        ):
            problems.append(
                "completion-report-duplicate: "
                + ", ".join(relative for relative, _ in attempts)
                + " report the same identities with the same outcomes; the same "
                "result staged twice is not two results"
            )
            continue
        final.update(attempts[-1][1])

    collisions = sorted(
        identity
        for index, (_, outcomes) in enumerate(reports)
        for identity in outcomes
        if any(
            identity in other
            for position, (_, other) in enumerate(reports)
            if position != index and frozenset(other) != frozenset(outcomes)
        )
    )
    for identity in sorted(set(collisions)):
        problems.append(
            f"completion-report-duplicate: {identity} is reported by two different "
            f"selections; a duplicate identity is a binary collision, not a retry"
        )

    if problems:
        raise Refused(problems)
    return final, sorted({relative for relative, _ in reports})


# ---------------------------------------------------------------------------
# The comparison
# ---------------------------------------------------------------------------


def approvals(plan: dict[str, Any], cell: dict[str, Any], today: date) -> tuple[bool, set[str]]:
    """This cell's live exact-skip approvals: `(approves_any, identities)`.

    An entry naming no `tests` approves any observed skip in its cell — the
    widest an approval can be and still be owned, dated, and attributable. An
    expired entry approves nothing, wherever it points: the planner refuses one
    outright, and a producer reading a carried plan must not be the only thing
    standing between an expired approval and a green cell.
    """
    policy = plan.get("skip_policy")
    entries = policy.get("entries", []) if isinstance(policy, dict) else []
    approves_any = False
    identities: set[str] = set()
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        if (
            entry.get("package") != cell["package"]
            or entry.get("environment") != cell["environment"]
            or entry.get("gate") != cell["gate"]
        ):
            continue
        expiry = entry.get("expiry")
        if expiry:
            try:
                if date.fromisoformat(str(expiry)) < today:
                    continue
            except ValueError:
                continue
        named = entry.get("tests")
        if named:
            identities.update(str(identity) for identity in named)
        else:
            approves_any = True
    return approves_any, identities


def comparison_problems(
    listing: dict[str, Any],
    observed: dict[str, str],
    cell: dict[str, Any],
    plan: dict[str, Any],
    today: date,
) -> list[str]:
    """Every reason the observed identities do not satisfy the expected ones."""
    expected = set(listing["tests"])
    problems: list[str] = []

    if not expected and not cell.get("companions"):
        problems.append(
            "completion-expected-empty: the listing expected no test at all and the "
            "plan records no reason for it; an empty expected set with a green, "
            "empty report is a drifted filter wearing evidence"
        )

    for identity in sorted(expected - set(observed)):
        problems.append(
            f"completion-test-missing: {identity} was expected on this target and no "
            f"report mentions it. A skip approval cannot excuse an absent test — an "
            f"observed skip is a decision, silence is a lost test"
        )
    for identity in sorted(set(observed) - expected):
        problems.append(
            f"completion-test-unexpected: {identity} reported a result that the "
            f"listing did not expect; the run and the listing disagree about what "
            f"this cell contains"
        )

    approves_any, approved = approvals(plan, cell, today)
    for identity in sorted(set(observed) & expected):
        outcome = observed[identity]
        if outcome == "fail":
            problems.append(f"completion-test-failed: {identity} failed")
        elif outcome == "skip" and not (approves_any or identity in approved):
            problems.append(
                f"completion-skip-unapproved: {identity} was skipped with no "
                f"unexpired approval in the plan's skip policy"
            )
    return problems


def companion_problems(cell: dict[str, Any], companions: dict[str, Any]) -> list[str]:
    problems = []
    for declared in cell.get("companions", []):
        name = declared.get("name") if isinstance(declared, dict) else None
        if not name:
            continue
        result = companions.get(name) if isinstance(companions, dict) else None
        if not isinstance(result, dict) or "outcome" not in result:
            problems.append(
                f"completion-companion-incomplete: the companion suite {name} is "
                f"attached to this cell and recorded no outcome; a declared suite "
                f"that did not complete leaves the cell unproven"
            )
        elif result["outcome"] != "success":
            problems.append(
                f"completion-companion-failed: the companion suite {name} reported "
                f"{result['outcome']!r}"
            )
    return problems


def backend_problems(cell: dict[str, Any], proofs: Any) -> list[str]:
    """Every required backend the proof document does not mark `proven`.

    `proofs` is `backend-proof verify`'s `backend-proofs.json`:
    `{backend: {"proven": bool, "executed": count}}` for each backend it was
    told to require. An absent document, an absent backend, and
    `proven: false` are the same verdict here — nothing proves the backend
    drove a test.
    """
    problems = []
    for backend in required_backends(cell):
        proof = proofs.get(backend) if isinstance(proofs, dict) else None
        if not isinstance(proof, dict) or proof.get("proven") is not True:
            problems.append(
                f"completion-backend-unproven: the {backend} backend is required of "
                f"this cell and nothing proves it drove a test; an absent backend "
                f"skips its suite and still prints PASS"
            )
    return problems


# ---------------------------------------------------------------------------
# The record
# ---------------------------------------------------------------------------


def completion_record(
    cell: dict[str, Any],
    package: dict[str, Any],
    plan: dict[str, Any],
    manifest: dict[str, Any] | None,
    reports: list[str],
    run: str,
    attempt: int,
) -> dict[str, Any]:
    record: dict[str, Any] = {
        "schema_version": schema.COMPLETION_RECORD_SCHEMA_VERSION,
        "package": cell["package"],
        "environment": cell["environment"],
        "gate": cell["gate"],
        "complete": True,
        "head": plan.get("head"),
        "run": run,
        "attempt": attempt,
        "reports": reports,
    }
    if manifest is not None:
        record["nextest_version"] = manifest["nextest_version"]
    if "build" in cell:
        record["build"] = cell["build"]
    if package.get("input_paths"):
        record["gate_inputs"] = sorted(package["input_paths"])
    companions = sorted(entry["name"] for entry in cell.get("companions", []))
    if companions:
        record["companions"] = companions
    backends = sorted(required_backends(cell))
    if backends:
        record["backends"] = backends
    return record


def validate(args: argparse.Namespace, today: date | None = None) -> dict[str, Any]:
    """The cell's completion record, or [`Refused`] with every reason it is not."""
    today = today or date.today()
    plan = read_json(args.plan, "completion-plan-unreadable", "the resolved plan")
    package_name, environment, gate = split_cell(args.cell)
    cell = cell_of(plan, package_name, environment, gate)
    package = package_of(plan, package_name)

    manifest: dict[str, Any] | None = None
    problems: list[str] = []
    reports: list[str] = []

    if gate in schema.BUILD_GATES:
        if args.expected_manifest is None:
            raise refuse(
                "completion-manifest-missing",
                f"cell {args.cell} runs tests and no --expected-manifest was given; "
                "nothing else can say what should have run",
            )
        manifest = read_json(
            args.expected_manifest,
            "completion-manifest-schema",
            "the expected-test manifest",
        )
        problems += manifest_problems(manifest, cell, package, args.target)
        if problems:
            raise Refused(problems)

        stage = staging_root(args.artifacts, cell)
        observed, reports = observed_outcomes(stage, staged_entries(stage, cell))
        problems += comparison_problems(
            manifest["packages"][package_name], observed, cell, plan, today
        )

    companions = (
        read_json(args.companions, "completion-companion-incomplete", "the companion results")
        if args.companions is not None and args.companions.is_file()
        else {}
    )
    proofs = (
        read_json(args.backend_proofs, "completion-backend-unproven", "the backend proofs")
        if args.backend_proofs is not None and args.backend_proofs.is_file()
        else {}
    )
    problems += companion_problems(cell, companions)
    problems += backend_problems(cell, proofs)

    if problems:
        raise Refused(problems)

    record = completion_record(
        cell, package, plan, manifest, reports, args.run, args.attempt
    )
    structural = schema.validate_completion_record(record)
    if structural:
        raise Refused(structural)
    return record


def parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--cell", required=True, help="<package>/<environment>/<gate>")
    parser.add_argument("--expected-manifest", type=Path)
    parser.add_argument(
        "--artifacts", type=Path, default=Path("."), help="the JUnit staging tree root"
    )
    parser.add_argument("--companions", type=Path)
    parser.add_argument("--backend-proofs", type=Path)
    parser.add_argument("--target", default="", help="the execution target triple")
    parser.add_argument("--run", default="")
    parser.add_argument("--attempt", type=int, default=1)
    parser.add_argument("--out", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        record = validate(args)
    except Refused as refusal:
        # A refusal must leave NO record at `--out`, including one an earlier
        # validation of the same path wrote: the upload step names the path, not
        # the outcome, and a stale record would be published as this run's proof.
        args.out.unlink(missing_ok=True)
        print(
            f"completion: {args.cell} is not proven and no record was written.",
            file=sys.stderr,
        )
        for problem in refusal.problems:
            print(f"  {problem}", file=sys.stderr)
        return 2 if refusal.infrastructure else 1
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(
        f"✅ {args.cell} complete: {len(record['reports'])} report(s) "
        f"match the expected listing"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
