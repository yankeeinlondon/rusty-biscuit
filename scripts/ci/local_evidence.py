#!/usr/bin/env python3
"""Record and verify local CI evidence carried by Git notes.

Two generations live here on purpose.

`verify_cells` is the current contract: it reads **every** note on every
`refs/notes/ci-local/<environment>` ref from the coverage boundary to the
outgoing head, gathers their observations, and resolves each
`{package, environment, gate}` cell among the candidates that qualify. It
returns a coded reason for every candidate it refused. Evidence from macOS and
from a prior WSL cross-check therefore combine in one run, and partial results
on one environment combine across commits and across observations of one
commit, which the environment-at-a-time predecessor could never express.
`observation_notes.py` is the writer of the version-3 collections it reads.

`record_scope` and `verify_scope` carry the *scope* receipt, a separate
document on `refs/notes/ci-local/scope`: what the planner selected for one
exact `{base, head, tree}`, which CI takes as its plan on an exact match and
otherwise recomputes.

`verified_environment` and `record` preserve the predecessor's CLI contract.
Current per-cell verification reads version-1 notes through `_read_legacy`
without calling that legacy CLI path. Spec section 3.6 makes those notes
exact-tree, pass-only, whole-environment, and never upgraded in place. Current
recorders use `record_cells`, which writes a version-2 receipt that
`observation_notes.py` carries into a collection.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable
from xml.etree import ElementTree

sys.path.insert(0, str(Path(__file__).resolve().parent))

import affected_scope  # noqa: E402  (needs the path insert above)
import schema  # noqa: E402


#: The version this module still *reads* through the legacy comparison below.
#: `record_cells` writes `schema.RECEIPT_SCHEMA_VERSION`.
SCHEMA_VERSION = schema.LEGACY_RECEIPT_SCHEMA_VERSION

ENVIRONMENTS = schema.ENVIRONMENTS
NOTES_PREFIX = "refs/notes/ci-local"

#: One ref for every host: scope is OS-independent. It sits inside the
#: `refs/notes/ci-local/*` pattern CI already fetches, and `scope` is not an
#: environment name, so `verify_cells`'s per-environment loop never reads it.
SCOPE_NOTES_REF = f"{NOTES_PREFIX}/scope"
assert "scope" not in ENVIRONMENTS

#: Tiers the JUnit staging manifest records, and therefore the gates a local
#: run can publish a measured outcome for. `lint` and `check` produce no report;
#: a `check` cell is satisfied only through its package's L1 receipt
#: (`affected_scope.check_evidence`).
RECORDABLE_GATES = ("L1", "L2", "browser")


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], check=True, capture_output=True, text=True, timeout=20
    ).stdout.strip()


def git_optional(*args: str) -> str | None:
    """`git(*args)`, or `None` when Git refuses — never a raised exception.

    Every caller here treats "Git cannot answer" as "no evidence", which is the
    fail-safe direction: the cell stays scheduled.
    """
    try:
        return git(*args)
    except (OSError, subprocess.SubprocessError):
        return None


def revision(ref: str) -> str:
    value = git("rev-parse", "--verify", ref)
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError(f"not a full Git object ID: {value}")
    return value


# ---------------------------------------------------------------------------
# Gate-input identity (spec section 3.3, Design Decision 4)
# ---------------------------------------------------------------------------


def gate_global_inputs(gate: str) -> list[str]:
    """The global inputs that belong to `gate`, as repository-relative paths.

    Derived from the planner's own per-gate classification rather than a second
    table: Design Decision 4 is "one classification, two consumers". The cell
    vocabulary (`L1`, `L2`, `browser`) collapses onto the planner's `test`
    gate, which is the gate those tiers are scheduled under.

    `Cargo.lock` is added here although the planner deliberately excludes it
    from *selection* (a lockfile edit must not schedule every package). For
    *equivalence* the opposite is true: a resolved dependency change makes an
    older result describe a different build, so the lockfile has to move the
    identity.

    The planner's orchestration files (`ORCHESTRATION_PATHS`) are left out for
    the opposite reason: they decide what CI runs, not what a local gate
    produces, so an older result under a different workflow still describes
    the same build.

    The Just files are not paths here at all. They enter the identity by
    recipe through [`just_gate_inputs`], exactly as they enter selection: the
    root justfile and `just/` hold some 7,000 lines, and a whole-file identity
    made an edit to a planning or notification recipe invalidate every
    published cell while the planner, reading the same edit by recipe,
    scheduled nothing.
    """
    planner_gate = planner_gate_of(gate)
    paths = set(affected_scope.GLOBAL_PATHS_ALL_GATES)
    paths |= set(affected_scope.GLOBAL_PATHS_BY_GATE[planner_gate])
    paths |= {prefix.rstrip("/") for prefix in affected_scope.GLOBAL_PREFIXES_ALL_GATES}
    paths.add(affected_scope.LOCKFILE_PATH)
    paths -= set(affected_scope.ORCHESTRATION_PATHS)
    paths -= {prefix.rstrip("/") for prefix in affected_scope.ORCHESTRATION_PREFIXES}
    return sorted(paths)


def planner_gate_of(gate: str) -> str:
    """The planner's gate a cell gate is scheduled under: `L1`, `L2`, and
    `browser` are all `test`."""
    return gate if gate in ("lint", "check") else "test"


def just_sources_at_ref(ref: str) -> dict[str, str]:
    """The root justfile and the `just/*.just` modules as they stand at `ref`.

    The same file set `affected_scope.just_sources` reads from disk, taken
    from the tree instead so two revisions can be compared without checking
    either out. A file absent at `ref` is absent from the result.
    """
    listing = git_optional("ls-tree", "-r", "--name-only", "--full-tree", ref, "--", "justfile", "just")
    sources: dict[str, str] = {}
    for path in sorted((listing or "").splitlines()):
        path = path.strip()
        is_module = path.startswith("just/") and path.endswith(".just") and path.count("/") == 1
        if path != "justfile" and not is_module:
            continue
        text = git_optional("show", f"{ref}:{path}")
        if text is not None:
            sources[path] = text
    return sources


def just_gate_inputs(gate: str, ref: str) -> list[str]:
    """The Just text that decides what `gate` executes, as it stands at `ref`.

    One rule with the planner's `just_change_gates`, read from the tree: the
    recipes reachable from the gate's CI entry recipes (`CI_RECIPES_BY_GATE`
    and `CI_RECIPES_ALL_GATES`) through header dependencies and `just <name>`
    calls, with comments and blank lines dropped, plus every line outside a
    recipe (settings, imports, assignments), which is conservatively global to
    every gate. A recipe nothing in CI reaches — `pre-push`, `cross-check`,
    the whole of `just/plan.just` — contributes nothing, so editing it leaves
    every published cell reusable.

    Order is preserved inside a recipe: swapping two commands is a different
    recipe, and a set would call it the same one.
    """
    recipes: dict[str, dict[str, Any]] = {}
    entries: list[str] = []
    for path, text in sorted(just_sources_at_ref(ref).items()):
        parsed, other = affected_scope.parse_just_recipes(text)
        recipes.update(parsed)
        entries.extend(f"{path}: {line}" for line in other)
    entries_recipes = (
        *affected_scope.CI_RECIPES_ALL_GATES,
        *affected_scope.CI_RECIPES_BY_GATE[planner_gate_of(gate)],
    )
    for name in sorted(affected_scope.just_recipe_closure(recipes, entries_recipes)):
        recipe = recipes.get(name)
        if recipe is not None:
            entries.extend(f"{name}: {line}" for line in recipe["lines"])
    return entries


def gate_input_identity(
    paths: list[str] | tuple[str, ...], ref: str, gate: str | None = None
) -> str:
    """The identity of `paths` as they stand at `ref`, plus the Just recipes
    `gate` executes there when a gate is named.

    Git's own hashing boundary is the rule for paths: the identity is over the
    `git ls-tree` entries — mode, type, object ID, path — for each input, so
    two trees agree exactly when Git says the content agrees. A path absent at
    `ref` contributes nothing, which is what makes an input that exists on
    neither side identical on both. The Just recipes are the one input read
    finer than a file: see [`just_gate_inputs`].

    ## Examples

    ```python
    before = gate_input_identity(["claudine/lib"], "HEAD~1", "L1")
    after = gate_input_identity(["claudine/lib"], "HEAD", "L1")
    ```
    """
    entries: list[str] = []
    for path in sorted({path for path in paths if path}):
        listing = git_optional("ls-tree", "-r", "--full-tree", ref, "--", path)
        if listing:
            entries.extend(line for line in listing.splitlines() if line.strip())
    ordered = sorted(set(entries))
    if gate is not None:
        ordered.extend(just_gate_inputs(gate, ref))
    return schema.identity("\n".join(ordered))


def cell_input_paths(plan: dict[str, Any], package: str, gate: str) -> list[str] | None:
    """Every path a cell's gate-input identity covers, or `None` if unknowable.

    `None` means the plan declared no build closure for the package, so no
    older receipt can be proven equivalent and only exact-tree reuse remains.
    """
    for entry in plan["packages"]:
        if entry["package"] != package:
            continue
        declared = entry.get("input_paths")
        if not declared:
            return None
        return [*declared, *gate_global_inputs(gate)]
    return None


# ---------------------------------------------------------------------------
# Version-2 verification
# ---------------------------------------------------------------------------


def load_plan(path: str) -> dict[str, Any]:
    document = schema.loads_strict(Path(path).read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise ValueError("the resolved plan must be a JSON object")
    return document


#: A base this many zeros long names no commit: the first run of a branch or of
#: the nightly schedule, which has nothing to diff against.
ZERO_REVISION = "0" * 40

#: The most annotated ancestors a bootstrap search considers, newest first.
#: Discovery is bounded by the notes that exist, never by history length.
ANCESTOR_NOTE_LIMIT = 500


def candidate_commits(base: str, head: str) -> list[str]:
    """Commits a note may be attached to, newest first.

    The outgoing head, everything it added over `base`, and `base` itself: the
    coverage boundary is a commit whose own proof is still valid for the head
    when its gate inputs are unchanged. Anything older than `base` describes
    work that is already upstream and is not consulted.

    An all-zero `base` is a bootstrap: there is no boundary to diff against, so
    the candidates are the annotated commits reachable from `head`. That search
    asks Git about each annotated commit rather than walking every commit, and
    keeps the newest [`ANCESTOR_NOTE_LIMIT`].
    """
    if base == ZERO_REVISION:
        return _annotated_ancestors(head)
    listing = git_optional("rev-list", f"{base}..{head}")
    commits = listing.splitlines() if listing else []
    if head not in commits:
        commits.insert(0, head)
    if base not in commits:
        commits.append(base)
    return commits


def _annotated_ancestors(head: str) -> list[str]:
    noted: set[str] = set()
    for environment in ENVIRONMENTS:
        listing = git_optional("notes", "--ref", f"{NOTES_PREFIX}/{environment}", "list")
        for line in (listing or "").splitlines():
            fields = line.split()
            if len(fields) == 2:
                noted.add(fields[1])
    reachable = [
        commit
        for commit in sorted(noted)
        if commit == head or git_optional("merge-base", "--is-ancestor", commit, head) is not None
    ]
    if not reachable:
        return [head]
    # Through stdin: a hundred annotated commits per environment would
    # otherwise overflow Windows' 32 KiB command line.
    try:
        ordered = subprocess.run(
            ["git", "rev-list", "--no-walk=sorted", "--stdin"],
            input="\n".join(reachable) + "\n",
            capture_output=True,
            text=True,
            check=True,
            timeout=20,
        ).stdout
    except (OSError, subprocess.SubprocessError):
        ordered = ""
    commits = (ordered.split() or reachable)[:ANCESTOR_NOTE_LIMIT]
    if head not in commits:
        commits.insert(0, head)
    return commits


def candidate_notes(environment: str, commits: list[str]) -> list[tuple[str, str]]:
    """Every note on `commits` for `environment`, in `commits` order.

    One `git notes list` per ref, then a read per hit: a branch of a hundred
    commits over four refs would otherwise cost four hundred `git notes show`
    calls that mostly answer "no note", and both the hook and CI pay it.
    """
    listing = git_optional("notes", "--ref", f"{NOTES_PREFIX}/{environment}", "list")
    if not listing:
        return []
    annotated: dict[str, str] = {}
    for line in listing.splitlines():
        fields = line.split()
        if len(fields) == 2:
            annotated[fields[1]] = fields[0]
    notes = []
    for commit in commits:
        blob = annotated.get(commit)
        if blob is None:
            continue
        text = git_optional("cat-file", "-p", blob)
        if text:
            notes.append((commit, text))
    return notes


class _Candidate:
    """One terminal cell result read from a note, before any plan is applied."""

    def __init__(
        self,
        environment: str,
        commit: str,
        order: int,
        document: dict[str, Any],
        cell: dict[str, Any],
        *,
        observation: dict[str, Any] | None = None,
        legacy: bool = False,
    ) -> None:
        self.environment = environment
        self.commit = commit
        self.order = order
        self.document = document
        self.cell = cell
        self.observation = observation
        self.legacy = legacy
        self.identifier = (
            observation["observation_id"]
            if observation is not None
            else f"{commit}:{environment}:{cell['package']}/{cell['gate']}/{cell.get('test_filter', '')}"
        )
        self.retries: set[str] = set()

    @property
    def exact_tree(self) -> str:
        return str(self.document.get("tree"))


def verify_cells(
    plan_path: str,
    base: str,
    head: str,
    report_probe: Callable[[dict[str, Any], dict[str, Any]], bool] | None = None,
) -> tuple[list[dict[str, Any]], list[str]]:
    """Every plan cell satisfied by published evidence, and why the rest were not.

    ## Returns

    `(accepted, rejections)`. Each accepted entry is a receipt cell widened
    with `environment`, `origin` (`local` for the outgoing tree, `prior-local`
    for an older head accepted through gate-input equivalence), `evidence`, and
    a rendered `measurements` string. Each rejection begins with a code from
    [`schema.REJECTIONS`].

    Missing, malformed, incompatible, stale, conflicting, and incomplete input
    all fail safe: the cell is simply not accepted, so it stays scheduled.

    `report_probe(report, observation)` answers whether an `artifact` report
    reference is still retrievable. Without one, an `artifact` reference is
    unavailable and the observation earns no credit.

    ## Notes

    Every note on an environment's ref reachable from the head and no older
    than the coverage boundary (`base`'s merge base; the annotated ancestors
    of the head when `base` is all zeros) is a candidate. Version-1 and -2
    notes are read as one observation per cell, version-3 notes as their
    observations. All candidates are gathered first and then resolved per
    `{package, environment, gate}`, so the outcome never depends on which
    environment or note was read first.

    A candidate **qualifies** for a plan cell when it is complete, credited to
    the ref it sits under, describes the commit it is attached to, and is on
    the head's exact tree or gate-input-equivalent to it. Among the qualifying
    candidates, one **supersedes** another when it names it in `retry_of`
    (same note), or when its commit is a descendant of the other's commit.
    Timestamps, fetch order, and publication order are never consulted. The
    surviving, unsuperseded candidates decide the cell: all passing credits
    the cell (newest first, lowest id on a tie); all failing leaves it
    scheduled; a mix of outcomes that nothing orders is reported as
    `conflicting-evidence` and the cell executes. So a newer complete failure
    is never hidden by an older pass, and a later passing retry restores
    reuse.

    A candidate that does not qualify (malformed, incomplete, mismatched,
    changed inputs, no declared closure) is reported with its code, naming its
    commit, unless a newer qualifying candidate already resolved the cell. A
    note that never mentions a wanted cell contributes nothing to it and is
    not a refusal, and a cell no candidate covers is reported nowhere: it
    simply stays scheduled. A version-1 note claims every wanted cell of its
    environment on its exact tree, pass-only.

    Equivalence recomputes the identity over **both** trees rather than
    trusting the identity the receipt stored. A stored string is an unverified
    claim by the host that wrote it; both trees are present locally, so the
    comparison can be made rather than believed.
    """
    try:
        plan = load_plan(plan_path)
    except (OSError, ValueError) as error:
        return [], [f"malformed-receipt: the resolved plan is unreadable: {error}"]

    problems = schema.validate_resolved_plan(plan)
    if problems:
        return [], [f"malformed-receipt: the resolved plan is invalid: {problems[0]}"]

    try:
        accepted, rejections, _ = _resolve(plan, base, head, report_probe)
    except _Unresolvable:
        return [], ["missing-receipt: the base/head pair does not resolve in this repository"]
    return accepted, rejections


def blocking_evidence(
    plan: dict[str, Any],
    head: str,
    report_probe: Callable[[dict[str, Any], dict[str, Any]], bool] | None = None,
) -> dict[tuple[str, str, str], str]:
    """The cells of `plan` whose qualifying evidence requires them to execute.

    The same resolution [`verify_cells`] makes, over every annotated ancestor
    of `head`, asked the other way round: which `{package, environment, gate}`
    cells are decided by an unsuperseded failure, or by passing and failing
    results nothing orders. Each maps to a sentence naming that evidence. A
    cell with passing evidence, or none, is absent.

    The planner asks this of the cells a nightly would inherit from its
    anchor, which no later step verifies; `plan` need only carry the `cells`,
    `packages`, and `builds` qualification reads.

    ## Errors

    Raises ``RuntimeError`` when `head` does not resolve here: a caller that
    cannot read the evidence must not credit coverage it may contradict.
    """
    try:
        return _resolve(plan, ZERO_REVISION, head, report_probe)[2]
    except _Unresolvable:
        raise RuntimeError(
            f"the head {head[:9]} does not resolve here, so no evidence about its "
            "inherited cells can be read"
        ) from None


class _Unresolvable(Exception):
    """The base/head pair names no commit in this repository."""


def _resolve(
    plan: dict[str, Any],
    base: str,
    head: str,
    report_probe: Callable[[dict[str, Any], dict[str, Any]], bool] | None,
) -> tuple[list[dict[str, Any]], list[str], dict[tuple[str, str, str], str]]:
    """`(accepted, rejections, blocked)` for `plan`'s cells; see [`verify_cells`].

    `blocked` maps each cell its qualifying evidence keeps scheduled to why.
    """
    rejections: list[str] = []
    try:
        head_sha = revision(head)
        boundary = (
            ZERO_REVISION if base == ZERO_REVISION else git("merge-base", base, head_sha)
        )
        head_tree = revision(f"{head_sha}^{{tree}}")
    except (OSError, ValueError, subprocess.SubprocessError):
        raise _Unresolvable from None

    wanted = {
        (cell["package"], cell["environment"], cell["gate"]) for cell in plan["cells"]
    }
    commits = candidate_commits(boundary, head_sha)
    order = {commit: index for index, commit in enumerate(commits)}

    candidates: list[_Candidate] = []
    for environment in ENVIRONMENTS:
        for note_commit, note_text in candidate_notes(environment, commits):
            candidates.extend(
                _read_note(
                    environment, note_commit, note_text, order[note_commit], head_tree,
                    wanted, rejections, plan, report_probe,
                )
            )

    by_plan_key: dict[tuple[str, str, str], list[tuple[_Candidate, str, str | None]]] = {}
    for candidate in candidates:
        for key, narrowed in _plan_keys(candidate, plan, wanted):
            label = "/".join(key)
            exact = candidate.exact_tree == head_tree
            origin, refusal = _qualification(
                candidate, key, narrowed, exact, plan, head_sha, label
            )
            by_plan_key.setdefault(key, []).append((candidate, origin, refusal))

    accepted: list[dict[str, Any]] = []
    blocked: dict[tuple[str, str, str], str] = {}
    ancestry: dict[tuple[str, str], bool] = {}
    for key in sorted(by_plan_key):
        entries = sorted(by_plan_key[key], key=lambda entry: (entry[0].order, entry[0].identifier))
        qualifying = [entry for entry in entries if entry[2] is None]
        first_resolved = min((entry[0].order for entry in qualifying), default=None)
        for candidate, _origin, refusal in entries:
            if refusal is not None and (first_resolved is None or candidate.order <= first_resolved):
                rejections.append(refusal)
        if not qualifying:
            continue
        survivors = [
            entry
            for entry in qualifying
            if not any(
                other is not entry and _supersedes(other[0], entry[0], ancestry)
                for other in qualifying
            )
        ]
        outcomes = {entry[0].cell["outcome"] for entry in survivors}
        if outcomes == {"pass", "fail"}:
            rejections.append(
                f"conflicting-evidence: {'/'.join(key)} has both passing and failing "
                "evidence for the same inputs and nothing orders them; it executes"
            )
            blocked[key] = "passing and failing results for its inputs that nothing orders"
            continue
        if outcomes == {"fail"}:
            failure = survivors[0][0]
            blocked[key] = (
                f"a later qualifying failure on {failure.commit[:9]} "
                f"({failure.identifier[:12]}) that no passing retry supersedes"
            )
            continue
        candidate, origin, _ = survivors[0]
        accepted.append(_accepted_candidate(candidate, key, origin, bool(candidate.cell.get("test_filter"))))

    return accepted, rejections, blocked


def _read_note(
    environment: str,
    note_commit: str,
    note_text: str,
    order: int,
    head_tree: str,
    wanted: set[tuple[str, str, str]],
    rejections: list[str],
    plan: dict[str, Any],
    report_probe: Callable[[dict[str, Any], dict[str, Any]], bool] | None,
) -> list[_Candidate]:
    """The terminal cell results of one note, or `[]` after recording why not."""
    try:
        document = schema.loads_strict(note_text)
    except ValueError as error:
        rejections.append(
            f"malformed-receipt: the {environment} note on {note_commit[:9]} "
            f"is not valid JSON ({error})"
        )
        return []
    if not isinstance(document, dict):
        rejections.append(
            f"malformed-receipt: the {environment} note on {note_commit[:9]} is not an object"
        )
        return []

    version = document.get("schema_version")
    if schema.is_version(version, schema.LEGACY_RECEIPT_SCHEMA_VERSION):
        return _read_legacy(document, environment, note_commit, order, head_tree, wanted, rejections)
    if schema.is_version(version, schema.COLLECTION_SCHEMA_VERSION):
        return _read_collection(
            document, environment, note_commit, order, plan, rejections, report_probe
        )

    receipt_valid = not schema.validate_receipt(document)
    cells, cell_rejections = schema.reusable_cells(document)
    rejections.extend(f"{reason} (note on {note_commit[:9]})" for reason in cell_rejections)
    if not receipt_valid:
        return []
    terminal_cells = cells
    if document["completion"] == "complete":
        terminal_cells = cells + [
            cell
            for cell in document["cells"]
            if cell["completion"] == "complete" and cell["outcome"] == "fail"
        ]
    if not terminal_cells:
        return []
    mismatch = _environment_mismatch(document, environment, note_commit) or (
        _revision_mismatch(document, note_commit)
    )
    if mismatch:
        rejections.append(mismatch)
        return []
    return [_Candidate(environment, note_commit, order, document, cell) for cell in terminal_cells]


def _read_legacy(
    document: dict[str, Any],
    environment: str,
    note_commit: str,
    order: int,
    head_tree: str,
    wanted: set[tuple[str, str, str]],
    rejections: list[str],
) -> list[_Candidate]:
    """Read a version-1 note under the narrow rules of spec section 3.6.

    Exact tree identity only, pass-only, and whole-environment — the same
    breadth the environment-at-a-time verifier granted, so a legacy receipt
    neither gains reach nor is silently upgraded. Its measurements render as
    [`schema.UNRECORDED_MEASUREMENT`] because the document has none. It claims
    every wanted cell of its environment.
    """
    mismatch = _environment_mismatch(document, environment, note_commit)
    if mismatch:
        rejections.append(mismatch)
        return []
    if document.get("tree") != head_tree:
        rejections.append(
            f"v1-not-equivalence-eligible: the {environment} version-1 receipt on "
            f"{note_commit[:9]} tested tree {str(document.get('tree'))[:9]}, and a "
            "version-1 receipt is reusable only on exact tree identity"
        )
        return []
    return [
        _Candidate(
            environment,
            note_commit,
            order,
            document,
            {
                "package": package,
                "gate": gate,
                "outcome": "pass",
                "completion": "complete",
            },
            legacy=True,
        )
        for package, env, gate in sorted(wanted)
        if env == environment
    ]


def _read_collection(
    document: dict[str, Any],
    environment: str,
    note_commit: str,
    order: int,
    plan: dict[str, Any],
    rejections: list[str],
    report_probe: Callable[[dict[str, Any], dict[str, Any]], bool] | None,
) -> list[_Candidate]:
    """Read a version-3 collection into its terminal, credit-eligible observations.

    A failure is kept even when its reports cannot be shown: hiding it would
    let an older pass stand in for a run that went red. A passing observation
    needs its reports.
    """
    problems = schema.validate_collection(document)
    if problems:
        rejections.append(f"{problems[0]} (note on {note_commit[:9]})")
        return []
    mismatch = _environment_mismatch(document, environment, note_commit) or (
        _revision_mismatch(document, note_commit)
    )
    if mismatch:
        rejections.append(mismatch)
        return []
    candidates = []
    for observation in document["observations"]:
        cell = observation["cell"]
        label = f"{cell['package']}/{environment}/{cell['gate']}"
        if observation["completion"] != "complete" or cell["completion"] != "complete":
            rejections.append(
                f"incomplete-run: {label} observation {observation['observation_id'][:12]} "
                f"is {cell['completion'] if observation['completion'] == 'complete' else observation['completion']}"
                f" (note on {note_commit[:9]})"
            )
            continue
        if cell["outcome"] == "pass":
            reason = _report_problem(observation, report_probe) or _proof_problem(observation)
            if reason:
                rejections.append(f"{reason} (note on {note_commit[:9]})")
                continue
        else:
            rejections.append(
                f"failed-cell: {label} failed; failing evidence is diagnostic only"
            )
        candidate = _Candidate(
            environment, note_commit, order, document, cell, observation=observation
        )
        links = {o["observation_id"]: o.get("retry_of") for o in document["observations"]}
        cursor = observation.get("retry_of")
        while cursor is not None and cursor not in candidate.retries:
            candidate.retries.add(cursor)
            cursor = links.get(cursor)
        candidates.append(candidate)
    return candidates


def _report_problem(
    observation: dict[str, Any],
    report_probe: Callable[[dict[str, Any], dict[str, Any]], bool] | None,
) -> str | None:
    """Why a passing observation's reports cannot be relied on, or `None`.

    A `host-dir` reference is checked byte for byte when its directory is on
    this host; a directory on another host cannot be checked from here and is
    accepted with its provenance. An `artifact` reference needs `report_probe`.
    """
    cell = observation["cell"]
    label = f"{cell['package']}/{cell['gate']} observation {observation['observation_id'][:12]}"
    host = observation["producer"]["host"]
    for report in observation["reports"]:
        if report["store"] == "artifact":
            if report_probe is None or not report_probe(report, observation):
                return (
                    f"observation-report-unavailable: {label} cites artifact "
                    f"{report['path']!r}, which cannot be retrieved"
                )
            continue
        directory = Path(host["report_dir"]).expanduser()
        if not directory.is_dir():
            continue
        path = directory / report["path"]
        try:
            content = path.read_bytes()
        except OSError:
            return (
                f"observation-report-unavailable: {label} cites {report['path']!r}, "
                f"which {directory} no longer retains"
            )
        if len(content) != report["bytes"] or hashlib.sha256(content).hexdigest() != report["sha256"]:
            return (
                f"observation-report-unavailable: {label} cites {report['path']!r}, "
                "whose bytes differ from the recorded digest"
            )
    return None


def _proof_problem(observation: dict[str, Any]) -> str | None:
    """Why a passing observation's proof documents do not match its digests.

    Only a `rig` or `seed` observation whose `report_dir` is on this host can be
    rechecked; another host's directory is accepted with its provenance, as a
    `host-dir` report is. A hosted observation's proof lives in workflow
    artifacts, which `report_probe` vouches for.
    """
    proof = observation.get("proof")
    producer = observation["producer"]
    if proof is None or producer["kind"] not in ("rig", "seed"):
        return None
    directory = Path(producer["host"].get("report_dir", "")).expanduser()
    if not producer["host"].get("report_dir") or not directory.is_dir():
        return None
    cell = observation["cell"]
    label = f"{cell['package']}/{cell['gate']} observation {observation['observation_id'][:12]}"
    for field, template in (
        ("expected_listing_digest", schema.PROOF_EXPECTED_FILE),
        ("completion_record_digest", schema.PROOF_COMPLETION_FILE),
    ):
        name = template.format(package=cell["package"], gate=cell["gate"])
        try:
            content = (directory / name).read_bytes()
        except OSError:
            return (
                f"observation-report-unavailable: {label} cites {name!r} as its "
                f"{field}, which {directory} no longer retains"
            )
        if hashlib.sha256(content).hexdigest() != proof[field]:
            return (
                f"observation-report-unavailable: {label} cites {name!r}, whose "
                f"bytes differ from its {field}"
            )
        if field == "completion_record_digest":
            try:
                record = schema.loads_strict(content.decode("utf-8"))
            except (UnicodeDecodeError, ValueError):
                return f"observation-proof-missing: {label} {name!r} is not a JSON document"
            if (
                not isinstance(record, dict)
                or record.get("complete") is not True
                or record.get("package") != cell["package"]
                or record.get("gate") != cell["gate"]
            ):
                return (
                    f"observation-proof-missing: {label} {name!r} does not certify "
                    "this cell as complete"
                )
    return None


def _plan_keys(
    candidate: _Candidate, plan: dict[str, Any], wanted: set[tuple[str, str, str]]
) -> list[tuple[tuple[str, str, str], bool]]:
    """The plan cells a candidate may stand in for, and whether it is narrowed.

    A narrowed test-input cell is the one place a receipt stands in for another
    environment (docs/cicd/test-inputs.md). It qualifies only for a plan cell
    with the identical package, gate, and `test_filter`, and the accepted entry
    is keyed by the PLAN's environment while its evidence names the host ref.
    """
    cell = candidate.cell
    if cell.get("test_filter"):
        return [
            ((planned["package"], planned["environment"], planned["gate"]), True)
            for planned in plan["cells"]
            if planned.get("test_filter") == cell["test_filter"]
            and planned["package"] == cell["package"]
            and planned["gate"] == cell["gate"]
            and (planned["package"], planned["environment"], planned["gate"]) in wanted
        ]
    key = (cell["package"], candidate.environment, cell["gate"])
    return [(key, False)] if key in wanted else []


def _qualification(
    candidate: _Candidate,
    key: tuple[str, str, str],
    narrowed: bool,
    exact: bool,
    plan: dict[str, Any],
    head_sha: str,
    label: str,
) -> tuple[str, str | None]:
    """`(origin, refusal)` for a candidate against one plan cell."""
    note_commit = candidate.commit
    if narrowed:
        if not exact:
            return "local", (
                f"gate-inputs-changed: {label} was narrowed to its test inputs on "
                f"{note_commit[:9]}, and a narrowed run proves only its exact tree"
            )
        origin = "local"
    elif exact:
        origin = "local"
    else:
        paths = cell_input_paths(plan, candidate.cell["package"], candidate.cell["gate"])
        if paths is None:
            return "prior-local", (
                f"gate-inputs-changed: {label} was tested on {note_commit[:9]}, "
                "and the plan declares no build closure for it, so equivalence "
                "with this head cannot be established"
            )
        if gate_input_identity(paths, note_commit, candidate.cell["gate"]) != gate_input_identity(
            paths, head_sha, candidate.cell["gate"]
        ):
            return "prior-local", (
                f"gate-inputs-changed: {label} was tested on {note_commit[:9]}, "
                "whose gate inputs differ from this head's"
            )
        origin = "prior-local"
    if candidate.observation is not None:
        refusal = _observation_refusal(candidate.observation, key, plan, label)
        if refusal:
            return origin, refusal
    return origin, None


def _observation_refusal(
    observation: dict[str, Any], key: tuple[str, str, str], plan: dict[str, Any], label: str
) -> str | None:
    """Why an observation does not describe the work a plan cell asks for."""
    planned = next(
        (
            cell
            for cell in plan["cells"]
            if (cell["package"], cell["environment"], cell["gate"]) == key
        ),
        None,
    )
    if planned is None:
        return None
    cell = observation["cell"]
    missing = sorted(set(planned.get("backends", [])) - set(cell["backends"]))
    if missing:
        return (
            f"incomplete-run: {label} observation {observation['observation_id'][:12]} "
            f"did not prove backend(s) {missing}"
        )
    wanted_features = _planned_features(plan, planned)
    if observation.get("features", "") != wanted_features:
        return (
            f"gate-inputs-changed: {label} observation {observation['observation_id'][:12]} "
            f"ran with features {observation.get('features', '')!r}, the plan's cell "
            f"uses {wanted_features!r}"
        )
    return None


def _planned_features(plan: dict[str, Any], planned: dict[str, Any]) -> str:
    """The isolated feature arguments the plan's build for `planned` was keyed on."""
    key = planned.get("build")
    for build in plan.get("builds", []):
        if build["key"] == key:
            return str(build["identity"].get("features", ""))
    return ""


def _supersedes(
    newer: _Candidate, older: _Candidate, ancestry: dict[tuple[str, str], bool]
) -> bool:
    """Whether `newer` replaces `older` for the same cell.

    Either `newer` retries `older` explicitly in the same note, or `newer`'s
    commit is a strict descendant of `older`'s. Two results on unordered
    commits, or on one commit with no link, supersede nothing.
    """
    if newer.commit == older.commit:
        return (
            newer.environment == older.environment
            and older.identifier in newer.retries
        )
    pair = (older.commit, newer.commit)
    if pair not in ancestry:
        ancestry[pair] = (
            git_optional("merge-base", "--is-ancestor", older.commit, newer.commit) is not None
        )
    return ancestry[pair]


def _accepted_candidate(
    candidate: _Candidate, key: tuple[str, str, str], origin: str, narrowed: bool
) -> dict[str, Any]:
    package, environment, gate = key
    if candidate.legacy:
        return {
            "package": package,
            "gate": gate,
            "environment": environment,
            "outcome": "pass",
            "completion": "complete",
            "origin": "local",
            "measurements": schema.UNRECORDED_MEASUREMENT,
            "evidence": {
                "ref": f"{NOTES_PREFIX}/{candidate.environment}",
                "schema_version": schema.LEGACY_RECEIPT_SCHEMA_VERSION,
            },
        }
    document = candidate.document
    if candidate.observation is not None:
        document = {"host": candidate.observation["producer"]["host"]}
    entry = _accepted_cell(candidate.cell, environment, origin, candidate.commit, document)
    entry["evidence"]["ref"] = f"{NOTES_PREFIX}/{candidate.environment}"
    if candidate.observation is not None:
        entry["evidence"]["observation"] = candidate.observation["observation_id"]
    if narrowed:
        entry["measurements"] += f", narrowed, on {candidate.environment}"
    return entry


def _accepted_cell(
    cell: dict[str, Any],
    environment: str,
    origin: str,
    note_commit: str,
    receipt: dict[str, Any],
) -> dict[str, Any]:
    counts = cell["counts"]
    return {
        **cell,
        "environment": environment,
        "origin": origin,
        "measurements": (
            f"{counts['total']} test(s), {counts['failed']} failed, "
            f"{cell['duration_s']}s"
        ),
        "evidence": {
            "ref": f"{NOTES_PREFIX}/{environment}",
            "commit": note_commit,
            "host": receipt["host"],
        },
    }


def _environment_mismatch(
    document: dict[str, Any], environment: str, note_commit: str
) -> str | None:
    """Why `document` may not be credited to the ref it was read from, or `None`.

    The ref names the environment a cell is credited to; the document names
    the environment the host claims it ran. A note filed under the wrong ref
    is credited to neither, or a macOS run could stand in for WSL.
    """
    declared = document.get("environment")
    if declared == environment:
        return None
    return (
        f"environment-mismatch: the note on {note_commit[:9]} under "
        f"{NOTES_PREFIX}/{environment} declares environment {declared!r}; "
        "it is credited to neither"
    )


def _revision_mismatch(document: dict[str, Any], note_commit: str) -> str | None:
    """Why `document` does not describe the commit it is attached to, or `None`.

    Equivalence compares the note's commit with the head, so a receipt whose
    declared head or tree is some other revision would be judged on inputs it
    never tested.
    """
    note_tree = git_optional("rev-parse", "--verify", f"{note_commit}^{{tree}}")
    if document.get("head") == note_commit and document.get("tree") == note_tree:
        return None
    return (
        f"revision-mismatch: the receipt on {note_commit[:9]} declares head "
        f"{str(document.get('head'))[:9]} and tree {str(document.get('tree'))[:9]}, "
        "which is not the commit it is attached to"
    )


# ---------------------------------------------------------------------------
# Scope evidence (fixes/2026-09-10-local-affected-scope, R1-R3)
# ---------------------------------------------------------------------------


def record_scope(plan_path: str, scope_path: str, base: str, head: str) -> str:
    """The scope receipt for one committed `base..head`, as canonical bytes.

    ## Errors

    Raises ``ValueError`` when a revision does not resolve or the assembled
    document would not validate. The comparison base may be an advanced PR
    target; verification still requires this exact base, head, and tree.
    """
    head_sha = revision(head)
    base_sha = revision(base)
    plan = load_plan(plan_path)
    document = {
        "schema_version": schema.SCOPE_RECEIPT_SCHEMA_VERSION,
        "base": base_sha,
        "head": head_sha,
        "tree": revision(f"{head_sha}^{{tree}}"),
        "plan_schema_version": plan.get("schema_version"),
        "plan": plan,
        "scope": load_scope(scope_path),
    }
    problems = schema.validate_scope_receipt(document)
    if problems:
        raise ValueError(f"the assembled scope receipt is invalid: {problems[0]}")
    return schema.canonical(document)


def verify_scope(
    base: str, head: str, event: str | None = None
) -> tuple[dict[str, Any] | None, str]:
    """The scope receipt that binds exactly `{base, head, head tree}`, or why not.

    Read from the event head only — scope binds to an exact head, so an older
    commit's note can never stand in. Checked in R3's order: schema, head,
    tree, base, event, then structure. Every miss fails safe: the caller
    calculates scope itself.

    `event` is the GitHub event the caller is running for. A receipt's plan
    schedules the environments of the event it was planned for, so a plan for
    another event — or for none — cannot stand in and misses as
    `scope-event-mismatch`.

    ## Returns

    `(receipt, "")` on a hit; `(None, reason)` on a miss, the reason beginning
    with a code from [`schema.SCOPE_REJECTIONS`].
    """
    try:
        head_sha = revision(head)
        head_tree = revision(f"{head_sha}^{{tree}}")
        base_sha = revision(base)
    except (OSError, ValueError, subprocess.SubprocessError):
        return None, "scope-missing: the base/head pair does not resolve in this repository"
    note = git_optional("notes", "--ref", SCOPE_NOTES_REF, "show", head_sha)
    if not note:
        return None, f"scope-missing: no scope receipt on {head_sha[:9]} under {SCOPE_NOTES_REF}"
    try:
        document = json.loads(note)
    except json.JSONDecodeError:
        return None, f"scope-malformed: the scope note on {head_sha[:9]} is not JSON"
    if not isinstance(document, dict):
        return None, f"scope-malformed: the scope note on {head_sha[:9]} is not an object"
    if document.get("schema_version") != schema.SCOPE_RECEIPT_SCHEMA_VERSION:
        return None, (
            f"scope-schema: the scope note on {head_sha[:9]} is version "
            f"{document.get('schema_version')!r}, this tool reads "
            f"{schema.SCOPE_RECEIPT_SCHEMA_VERSION}"
        )
    for field, expected, code in (
        ("head", head_sha, "scope-head-mismatch"),
        ("tree", head_tree, "scope-tree-mismatch"),
        ("base", base_sha, "scope-base-mismatch"),
    ):
        declared = document.get(field)
        if declared != expected:
            return None, (
                f"{code}: the scope note on {head_sha[:9]} declares {field} "
                f"{str(declared)[:9]}, the event's is {expected[:9]}"
            )
    if event is not None:
        plan = document.get("plan")
        planned_event = plan.get("event") if isinstance(plan, dict) else None
        if planned_event != event:
            return None, (
                f"scope-event-mismatch: the scope note on {head_sha[:9]} was planned "
                f"for event {planned_event!r}, this run is {event!r}"
            )
    problems = schema.validate_scope_receipt(document)
    if problems:
        return None, problems[0]
    return document, ""


# ---------------------------------------------------------------------------
# Version-2 recording
# ---------------------------------------------------------------------------


def junit_counts(report: Path) -> tuple[dict[str, int], list[str]] | None:
    """Counts and failing test names from a staged JUnit report.

    ## Returns

    `None` when the report is absent or unparseable, which the caller turns
    into a non-reusable cell rather than into invented measurements.
    """
    try:
        root = ElementTree.parse(report).getroot()
    except (OSError, ElementTree.ParseError):
        return None
    suites = [root] if root.tag == "testsuite" else list(root.iter("testsuite"))
    totals = {"total": 0, "passed": 0, "failed": 0, "skipped": 0, "errored": 0}
    for suite in suites:
        total = int(suite.get("tests", 0))
        failed = int(suite.get("failures", 0))
        errored = int(suite.get("errors", 0))
        skipped = int(suite.get("skipped", 0))
        totals["total"] += total
        totals["failed"] += failed
        totals["errored"] += errored
        totals["skipped"] += skipped
        totals["passed"] += total - failed - errored - skipped
    failing = []
    for case in root.iter("testcase"):
        if case.find("failure") is not None or case.find("error") is not None:
            name = "::".join(filter(None, (case.get("classname"), case.get("name"))))
            failing.append(name or "<unnamed>")
    return totals, failing


def staged_records(stage: Path) -> list[dict[str, Any]]:
    """The JUnit staging manifest `just _stage_junit` appends to."""
    manifest = stage / "manifest.jsonl"
    if not manifest.is_file():
        return []
    records = []
    for line in manifest.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict) and record.get("tier") in RECORDABLE_GATES:
            records.append(record)
    return records


#: Gates whose result is meaningless without proof that a terminal backend
#: actually drove a test. A Level 2 suite whose backend is absent *skips*, and
#: nextest prints PASS in about 0.02 s — indistinguishable from evidence unless
#: the backend proof is read.
BACKEND_PROVEN_GATES = ("L2",)

#: Written by `test-toolkit` while `BISCUIT_TEST_REQUIRED_BACKENDS` is set: one
#: `{backend, test, decision}` record per gate decision.
BACKEND_EXECUTIONS_FILE = "backend-executions.jsonl"

#: Written by `backend-proof verify` beside the execution log: `{backend:
#: {"proven": bool, "executed": count}}` for every backend it was told to
#: require. `completion.py --backend-proofs` reads it; the hook does not, since
#: it derives the same verdict from the execution records above.
BACKEND_PROOFS_FILE = "backend-proofs.json"

#: Written by `just ci-local`: the backends it required for each invocation,
#: which is the only per-cell attribution available — the execution records
#: name a test, not a package.
GATE_BACKENDS_FILE = "gate-backends.jsonl"


def _jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        return []
    records = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            records.append(record)
    return records


def executed_backends(stage: Path) -> set[str]:
    """The backends that actually ran at least one test in this staging directory."""
    return {
        record["backend"]
        for record in _jsonl(stage / BACKEND_EXECUTIONS_FILE)
        if record.get("decision") == "run" and record.get("backend")
    }


def required_backends(stage: Path) -> dict[tuple[str, str], list[str]]:
    """The backends each `{package, gate}` invocation required."""
    return {
        (record["package"], record["gate"]): list(record.get("backends", []))
        for record in _jsonl(stage / GATE_BACKENDS_FILE)
        if record.get("package") and record.get("gate")
    }


def receipt_cell(
    record: dict[str, Any],
    stage: Path,
    plan: dict[str, Any],
    head: str,
    proven: set[str] | None = None,
    required: dict[tuple[str, str], list[str]] | None = None,
) -> dict[str, Any]:
    """One measured cell of a local run.

    A gate whose report is missing or unreadable is recorded `partial`: the
    run has an exit code but no attributable outcome, so it must not stand in
    for a CI execution. The same applies to a failure whose report names no
    failing test — the receipt would assert a failure nothing can be acted on,
    and to a backend-proven gate no backend actually drove.
    """
    gate = record["tier"]
    package = record["package"]
    paths = cell_input_paths(plan, package, gate)
    measured = junit_counts(stage / record.get("xml", ""))
    counts = {field: 0 for field in schema.COUNT_FIELDS}
    failing: list[str] = []
    completion = "partial"
    if measured is not None:
        counts, failing = measured
        completion = "complete"
    outcome = "pass" if record.get("exit_code", 0) == 0 else "fail"
    if outcome == "fail" and not failing:
        completion = "partial"

    wanted = (required or {}).get((package, gate), [])
    backends = sorted(set(wanted) & (proven or set()))
    if gate in BACKEND_PROVEN_GATES and not backends:
        completion = "partial"

    cell = {
        "package": package,
        "gate": gate,
        "outcome": outcome,
        "exit_code": int(record.get("exit_code", 0)),
        "completion": completion,
        "counts": counts,
        "duration_s": record.get("duration_s", 0),
        "gate_input_identity": (
            gate_input_identity(paths, head, gate) if paths else schema.identity(f"{package}/{gate}")
        ),
        "backends": backends,
        "report": record.get("xml", ""),
    }
    if len(failing) > schema.FAILURE_DETAIL_LIMIT:
        cell["failed_tests"] = sorted(failing)[: schema.FAILURE_DETAIL_LIMIT]
        cell["failure_detail_truncated"] = True
    elif failing:
        cell["failed_tests"] = sorted(failing)
    return cell


def record_cells(
    plan_path: str,
    stage_dir: str,
    environment: str,
    base: str,
    head: str,
    completion: str = "complete",
    report_dir: str = "",
    prior_receipt_path: str = "",
) -> str:
    """The version-2 receipt for a finished local run, as canonical bytes.

    `report_dir` is where the producing host retains the staged reports after
    the run — the receipt's `host.report_dir` (spec section 2). It is required
    even though the parameter has a default: the hook is the only producer and
    its staging directory is deleted on exit, so a receipt is published only
    for reports that were already copied somewhere durable.

    ## Errors

    Raises ``ValueError`` when a revision does not resolve, when the run
    staged no recordable gate, when `report_dir` is empty or does not retain
    every report a complete cell names, or when
    the assembled document would not validate — publishing an invalid receipt
    is worse than publishing none, because CI would reject it silently later.
    """
    head_sha = revision(head)
    plan = load_plan(plan_path)
    problems = schema.validate_resolved_plan(plan)
    if problems:
        raise ValueError(f"the resolved plan is invalid: {problems[0]}")

    stage = Path(stage_dir)
    proven = executed_backends(stage)
    required = required_backends(stage)
    # A narrowed test-input cell may be planned on another environment than
    # this host's: its question does not depend on the OS, so the run is
    # recorded here WITH its filter, and verification credits it to the plan's
    # cell only (docs/cicd/test-inputs.md). Without the filter it would read as
    # this host's whole tier.
    narrowed = {
        (cell["package"], cell["gate"]): cell["test_filter"]
        for cell in plan["cells"]
        if cell.get("test_filter")
    }
    cells = []
    for record in staged_records(stage):
        cell = receipt_cell(record, stage, plan, head_sha, proven, required)
        narrow = narrowed.get((record["package"], record["tier"]))
        if narrow:
            cell["test_filter"] = narrow
        cells.append(cell)
    if not cells:
        raise ValueError(
            f"the run staged no L1/L2/browser report under {stage_dir}; there is "
            "nothing to publish"
        )
    if not report_dir.strip():
        raise ValueError(
            "record-cells needs --report-dir: the receipt names where the reports "
            "are retained, and the staging directory does not outlive the hook"
        )
    retained = Path(report_dir)
    current = json.loads(
        assemble_receipt(
            plan, environment, base, head_sha, cells, completion, report_dir
        )
    )
    document = merge_prior_receipt(current, prior_receipt_path)
    for cell in document["cells"]:
        if cell["completion"] == "complete" and not (retained / cell["report"]).is_file():
            raise ValueError(
                f"{report_dir} does not retain {cell['report']}; a receipt must not "
                "name reports that are not there"
            )
    problems = schema.validate_receipt(document)
    if problems:
        raise ValueError(f"the merged receipt is invalid: {problems[0]}")
    return schema.canonical(document)


def assemble_receipt(
    plan: dict[str, Any],
    environment: str,
    base: str,
    head_sha: str,
    cells: list[dict[str, Any]],
    completion: str,
    report_dir: str,
    host: dict[str, str] | None = None,
) -> str:
    """The receipt document, validated, as the bytes a Git note carries.

    `report_dir` is recorded verbatim; nothing here invents a location when it
    is empty, because a path no reports were copied to is provenance for nothing.

    ## Errors

    Raises ``ValueError`` when the document would not validate — publishing an
    invalid receipt is worse than publishing none, because CI would reject it
    silently later.
    """
    if host is None and not report_dir.strip():
        raise ValueError("a receipt must name the directory that retains its reports")
    document = {
        "schema_version": schema.RECEIPT_SCHEMA_VERSION,
        "environment": environment,
        "base": revision(base),
        "head": head_sha,
        "tree": revision(f"{head_sha}^{{tree}}"),
        "scope_identity": schema.identity(schema.canonical(plan)),
        "completion": completion,
        "host": host
        or {
            "os": platform.system(),
            "kernel": f"{platform.system()} {platform.release()}",
            "report_dir": report_dir,
        },
        "cells": cells,
    }
    problems = schema.validate_receipt(document)
    if problems:
        raise ValueError(f"the assembled receipt is invalid: {problems[0]}")
    return schema.canonical(document)


def merge_prior_receipt(current: dict[str, Any], prior_receipt_path: str) -> dict[str, Any]:
    """Merge a compatible same-head receipt into a retry's partial result.

    The current run wins for every `{package, gate}` it executed. A prior
    receipt is accepted only when its complete execution context and report
    directory match, so carried cells retain truthful provenance.

    ## Errors

    Raises ``ValueError`` when a non-empty prior receipt is malformed,
    invalid, or belongs to another execution context.
    """
    if not prior_receipt_path:
        return current
    text = Path(prior_receipt_path).read_text(encoding="utf-8").strip()
    if not text:
        return current
    try:
        prior = schema.loads_strict(text)
    except ValueError as error:
        raise ValueError(f"the prior receipt is not valid JSON ({error})") from error
    if not isinstance(prior, dict):
        raise ValueError("the prior receipt is not an object")
    problems = schema.validate_receipt(prior)
    if problems:
        raise ValueError(f"the prior receipt is invalid: {problems[0]}")
    bindings = (
        "schema_version",
        "environment",
        "base",
        "head",
        "tree",
        "scope_identity",
        "completion",
        "host",
    )
    mismatched = [field for field in bindings if prior.get(field) != current.get(field)]
    if mismatched:
        raise ValueError(
            "the prior receipt does not match the current " + ", ".join(mismatched)
        )
    merged = {
        (cell["package"], cell["gate"]): cell
        for cell in prior["cells"]
    }
    merged.update(
        {(cell["package"], cell["gate"]): cell for cell in current["cells"]}
    )
    current["cells"] = [merged[key] for key in sorted(merged)]
    return current


# ---------------------------------------------------------------------------
# Cross-check publication (spec section 3.8, Design Decision 7)
# ---------------------------------------------------------------------------


def cross_check_rejection(
    tested_tree: str, head_tree: str, remote_dirty: bool, run_filters: str
) -> str | None:
    """Why a `cross-check` run may not become a receipt, or `None` if it may.

    `scripts/cross-check.sh` ships the developer's *local* tree — including
    uncommitted work — to a standing clone. That is the right thing for a smoke
    run and the wrong thing for evidence: only a run whose tested tree is the
    outgoing head's tree, on a clean remote worktree, with no test filter
    narrowing the cell, describes the cell CI would otherwise execute.
    """
    if not tested_tree or tested_tree != head_tree:
        return (
            f"the remote tested tree {tested_tree[:9] or '<unknown>'} is not this "
            f"head's tree {head_tree[:9]}; a patched or older tree is not evidence "
            "for this head"
        )
    if remote_dirty:
        return "the remote worktree was dirty, so the run did not test a known tree"
    if run_filters.strip():
        return (
            f"the run was narrowed by {run_filters.strip()!r}; a filtered run does "
            "not cover the cell it would be published as"
        )
    return None


#: The rig environments a `cross-check` run can be recorded for, with the host
#: OS and default label each is described by. WSL2 runs Linux code paths and is
#: never filed under `windows-latest`, nor the reverse.
CROSS_CHECK_HOSTS = {
    "wsl2-ubuntu": ("Linux", "WSL2 (cross-check archive mode)"),
    "windows-latest": ("Windows", "native Windows (cross-check archive mode)"),
}


def record_cross_check(
    plan_path: str,
    package: str,
    report: str,
    exit_code: int,
    duration_s: int,
    tested_tree: str,
    remote_dirty: bool,
    run_filters: str,
    base: str,
    head: str,
    environment: str = "wsl2-ubuntu",
    host_label: str = "",
    build_key: str = "",
    build_digest: str = "",
) -> str:
    """The receipt for a qualifying `cross-check` run.

    `environment` is the rig the run executed on: `wsl2-ubuntu` (a Linux guest)
    or `windows-latest` (native Windows). A result is only ever recorded for
    the environment it ran on; the plan must carry that environment's cell.
    `report` sits inside the directory the receipt names as retaining it.

    `build_key` and `build_digest` are the manifest the remote consumer
    VERIFIED, not the plan's expectation of it: a receipt that named a key
    nothing checked would be a claim rather than evidence. Both or neither —
    a half-named build is recorded as no build at all.

    ## Errors

    Raises ``ValueError`` with the exact publication reason when the run does
    not qualify, so the caller can print it rather than silently publishing
    nothing.
    """
    if environment not in CROSS_CHECK_HOSTS:
        raise ValueError(
            f"{environment} is not a rig environment; a cross-check run can only "
            f"be recorded for {sorted(CROSS_CHECK_HOSTS)}"
        )
    head_sha = revision(head)
    head_tree = revision(f"{head_sha}^{{tree}}")
    refusal = cross_check_rejection(tested_tree, head_tree, remote_dirty, run_filters)
    if refusal is not None:
        raise ValueError(refusal)

    plan = load_plan(plan_path)
    problems = schema.validate_resolved_plan(plan)
    if problems:
        raise ValueError(f"the resolved plan is invalid: {problems[0]}")
    if not any(
        cell["package"] == package
        and cell["environment"] == environment
        and cell["gate"] == "L1"
        for cell in plan["cells"]
    ):
        raise ValueError(
            f"the resolved plan has no {package}/{environment}/L1 cell for this "
            "head, so the run proves nothing the plan asked for"
        )

    stage = Path(report).parent
    cell = receipt_cell(
        {
            "tier": "L1",
            "package": package,
            "xml": Path(report).name,
            "exit_code": exit_code,
            "duration_s": duration_s,
        },
        stage,
        plan,
        head_sha,
    )
    if cell["completion"] != "complete":
        raise ValueError(
            f"the remote run produced no usable report for {package}; nothing is "
            "published"
        )
    if build_key and build_digest:
        cell["build"] = {"key": build_key, "digest": build_digest}
    return assemble_receipt(
        plan,
        environment,
        base,
        head_sha,
        [cell],
        "complete",
        "",
        host={
            "os": CROSS_CHECK_HOSTS[environment][0],
            "kernel": host_label or CROSS_CHECK_HOSTS[environment][1],
            "report_dir": str(stage),
        },
    )


# ---------------------------------------------------------------------------
# Version-1 compatibility (spec section 3.6)
# ---------------------------------------------------------------------------


def expected_evidence(
    scope: dict[str, Any], base: str, head: str, environment: str
) -> dict[str, Any]:
    source = sorted(scope.get("source_packages", []))
    reverse = sorted(scope.get("reverse_dependencies", []))
    matrix = {
        entry["package"]: entry
        for entry in scope.get("matrix", [])
        if entry["package"] in source
    }

    def has_l1(entry: dict[str, Any]) -> bool:
        if "test" not in entry.get("gates", []):
            return False
        if environment == "wsl2-ubuntu":
            return bool(entry.get("wsl"))
        return environment in entry.get("native_environments", [])

    l1 = sorted(name for name, entry in matrix.items() if has_l1(entry))
    l2 = sorted(
        name
        for name, entry in matrix.items()
        if "L2" in entry.get("tiers", [])
        and environment in entry.get("l2_environments", [])
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "base": revision(base),
        "head": revision(head),
        "tree": revision(f"{head}^{{tree}}"),
        "environment": environment,
        "source_packages": source,
        "reverse_dependencies": reverse,
        "l1_packages": l1,
        "l2_packages": l2,
    }


def load_scope(path: str) -> dict[str, Any]:
    document = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise ValueError("scope must be a JSON object")
    return document


def record(scope_path: str, base: str, head: str, environment: str) -> str:
    """A version-1 receipt. Retained to read and test notes already in the wild."""
    if git("merge-base", base, head) != revision(base):
        raise ValueError("the tested base must be an ancestor of the outgoing head")
    return json.dumps(
        expected_evidence(load_scope(scope_path), base, head, environment),
        sort_keys=True,
        separators=(",", ":"),
    )


def verified_environment(scope_path: str, base: str, head: str) -> str:
    """The first environment whose version-1 note matches the legacy scope.

    Superseded by [`verify_cells`], which answers per cell across every
    environment at once. Kept because version-1 notes bind to exact heads and
    must keep verifying until those heads are gone.
    """
    scope = load_scope(scope_path)
    # PR base.sha may have advanced since the branch diverged. The hook tests
    # from that divergence point; CI still compares its independently computed
    # package/tier scope, so additional upstream changes cannot widen coverage.
    try:
        base = git("merge-base", base, head)
    except (OSError, subprocess.SubprocessError):
        return ""
    for environment in ENVIRONMENTS:
        try:
            note = git(
                "notes", "--ref", f"{NOTES_PREFIX}/{environment}", "show", head
            )
            actual = json.loads(note)
            expected = expected_evidence(scope, base, head, environment)
        except (OSError, subprocess.SubprocessError, ValueError, json.JSONDecodeError):
            continue
        if actual == expected:
            return environment
    return ""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    recorder = subparsers.add_parser(
        "record", help="emit a version-1 receipt from a legacy scope document"
    )
    recorder.add_argument("--scope", required=True)
    recorder.add_argument("--base", required=True)
    recorder.add_argument("--head", required=True)
    recorder.add_argument("--environment", required=True, choices=ENVIRONMENTS)

    cells = subparsers.add_parser(
        "record-cells", help="emit a version-2 receipt from a finished local run"
    )
    cells.add_argument("--plan", required=True)
    cells.add_argument("--stage", required=True)
    cells.add_argument("--base", required=True)
    cells.add_argument("--head", required=True)
    cells.add_argument("--environment", required=True, choices=ENVIRONMENTS)
    cells.add_argument(
        "--completion", default="complete", choices=list(schema.COMPLETIONS)
    )
    cells.add_argument(
        "--report-dir",
        required=True,
        help="the durable directory the staged reports were copied to",
    )
    cells.add_argument(
        "--prior-receipt",
        default="",
        help="compatible receipt whose unexecuted same-head cells must be retained",
    )

    cross = subparsers.add_parser(
        "cross-check", help="emit a receipt for a qualifying cross-check run"
    )
    cross.add_argument("--plan", required=True)
    cross.add_argument("--package", required=True)
    cross.add_argument("--report", required=True)
    cross.add_argument("--exit-code", type=int, required=True)
    cross.add_argument("--duration", type=int, default=0)
    cross.add_argument("--tested-tree", required=True)
    cross.add_argument("--remote-dirty", action="store_true")
    cross.add_argument("--run-filters", default="")
    cross.add_argument("--base", required=True)
    cross.add_argument("--head", required=True)
    cross.add_argument("--environment", default="wsl2-ubuntu", choices=ENVIRONMENTS)
    cross.add_argument("--host-label", default="")
    cross.add_argument("--build-key", default="")
    cross.add_argument("--build-digest", default="")

    scope_recorder = subparsers.add_parser(
        "scope-record", help="emit a scope receipt for one committed base..head"
    )
    scope_recorder.add_argument("--plan", required=True)
    scope_recorder.add_argument("--scope", required=True, help="the legacy scope projection")
    scope_recorder.add_argument("--base", required=True)
    scope_recorder.add_argument("--head", required=True)

    scope_verifier = subparsers.add_parser(
        "scope-verify",
        help="write the plan and scope a matching scope receipt carries; exit 3 on a miss",
    )
    scope_verifier.add_argument("--base", required=True)
    scope_verifier.add_argument("--head", required=True)
    scope_verifier.add_argument("--plan-out", required=True, metavar="FILE")
    scope_verifier.add_argument("--scope-out", required=True, metavar="FILE")
    scope_verifier.add_argument(
        "--reason-out", metavar="FILE", help="write the coded miss reason here"
    )
    scope_verifier.add_argument(
        "--event",
        choices=affected_scope.EVENT_NAMES,
        help="the GitHub event this run is for; a receipt planned for another misses",
    )

    verifier = subparsers.add_parser("verify")
    verifier.add_argument("--scope", help="legacy scope document (version-1 path)")
    verifier.add_argument("--plan", help="resolved plan (version-2 path)")
    verifier.add_argument("--base", required=True)
    verifier.add_argument("--head", required=True)
    verifier.add_argument(
        "--cells",
        action="store_true",
        help="emit the accepted per-cell set as JSON instead of one environment name",
    )
    verifier.add_argument(
        "--rejections",
        metavar="FILE",
        help="write the coded rejection reasons here",
    )
    verifier.add_argument(
        "--artifact-repo",
        metavar="OWNER/REPO",
        help="vouch for hosted `artifact` report references by fetching them from "
        "this repository's workflow runs (needs GH_TOKEN); without it a hosted "
        "observation earns no credit",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.command == "record":
        print(record(args.scope, args.base, args.head, args.environment))
        return
    if args.command == "record-cells":
        print(
            record_cells(
                args.plan,
                args.stage,
                args.environment,
                args.base,
                args.head,
                args.completion,
                args.report_dir,
                args.prior_receipt,
            )
        )
        return
    if args.command == "scope-record":
        print(record_scope(args.plan, args.scope, args.base, args.head))
        return
    if args.command == "scope-verify":
        receipt, reason = verify_scope(args.base, args.head, args.event)
        if receipt is None:
            if args.reason_out:
                Path(args.reason_out).write_text(reason + "\n", encoding="utf-8")
            print(f"scope-verify: {reason}", file=sys.stderr)
            raise SystemExit(3)
        Path(args.plan_out).write_text(schema.canonical(receipt["plan"]), encoding="utf-8")
        Path(args.scope_out).write_text(
            schema.canonical(affected_scope.legacy_scope_document(receipt["plan"])),
            encoding="utf-8",
        )
        return
    if args.command == "cross-check":
        try:
            print(
                record_cross_check(
                    args.plan,
                    args.package,
                    args.report,
                    args.exit_code,
                    args.duration,
                    args.tested_tree,
                    args.remote_dirty,
                    args.run_filters,
                    args.base,
                    args.head,
                    args.environment,
                    args.host_label,
                    args.build_key,
                    args.build_digest,
                )
            )
        except (OSError, ValueError) as refusal:
            print(f"cross-check: publishing no receipt — {refusal}", file=sys.stderr)
            raise SystemExit(3) from refusal
        return
    if args.cells:
        if not args.plan:
            raise SystemExit("verify --cells requires --plan")
        probe = None
        if args.artifact_repo:
            import hosted_observations  # noqa: PLC0415  (only the nightly reaches the network)

            probe = hosted_observations.gh_probe(args.artifact_repo)
        accepted, rejections = verify_cells(
            args.plan, args.base, args.head, report_probe=probe
        )
        if args.rejections:
            Path(args.rejections).write_text(
                json.dumps(rejections, indent=0), encoding="utf-8"
            )
        print(json.dumps(accepted, separators=(",", ":")))
        return
    if not args.scope:
        raise SystemExit("verify without --cells requires --scope")
    print(verified_environment(args.scope, args.base, args.head))


if __name__ == "__main__":
    main()
