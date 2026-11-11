//! Freeze the Python evidence contract as parity fixtures.
//!
//! `scripts/ci/schema.py` and `scripts/ci/observation_notes.py` are the
//! executable contract the TypeScript port in `src/contracts/` must match byte
//! for byte. This command runs the real Python (`python3 -I`, no shell) over a
//! fixed set of inputs and writes what it answers into
//! `tests/fixtures/parity/`: the raw input text, the canonical bytes, the
//! SHA-256 identities, the adapted collections, and the exact problem strings.
//!
//! Usage:
//!
//!     pnpm --dir tools/evidence-store exec tsx cli/capture-parity.ts          # regenerate
//!     pnpm --dir tools/evidence-store exec tsx cli/capture-parity.ts --check  # compare, exit 1 on drift
//!
//! The Python writes only into a fresh temporary directory; this command then
//! mirrors that directory onto `tests/fixtures/parity/` (or, with `--check`,
//! only compares). It never touches Git, notes, the network, or any other path.
//! `HOME`/`USERPROFILE` point at a temporary directory so `~` expansion in the
//! v2 receipt adapter is deterministic.

import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const PYTHON_PROGRAM = String.raw`
import copy
import json
import os
import sys
import types
from pathlib import Path

ROOT = Path(sys.argv[1])
OUT = Path(sys.argv[2])
SCRATCH = Path(sys.argv[3])
sys.path.insert(0, str(ROOT / "scripts" / "ci"))

import schema  # noqa: E402
import observation_notes  # noqa: E402
import hosted_observations  # noqa: E402
import local_evidence  # noqa: E402
import test_hosted_observations  # noqa: E402

SHA_A = "a" * 40
SHA_B = "b" * 40
SHA_T = "c" * 40
DELETE = object()
CASES = []


def emit(relative, data):
    path = OUT / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(data, str):
        data = data.encode("utf-8")
    path.write_bytes(data)


def pretty(document):
    return json.dumps(document, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def codes(problems):
    return [problem.split(":", 1)[0] for problem in problems]


def collection_record(document):
    text = schema.canonical(document)
    return {
        "identity": schema.identity(text),
        "observation_ids": [
            schema.observation_identity(o) for o in document["observations"]
        ],
        "problems": schema.validate_collection(json.loads(text)),
    }, text


# ---------------------------------------------------------------------------
# Authentic shapes (scripts/ci/test_schema.py receipt/receipt_cell/observation)
# ---------------------------------------------------------------------------


def receipt_cell(**overrides):
    record = {
        "package": "claudine",
        "gate": "L1",
        "outcome": "pass",
        "exit_code": 0,
        "completion": "complete",
        "counts": {"total": 10, "passed": 9, "failed": 0, "skipped": 1, "errored": 0},
        "duration_s": 35,
        "gate_input_identity": "deadbeefcafe",
        "backends": [],
        "report": "claudine-L1.xml",
    }
    record.update(overrides)
    return record


def receipt(**overrides):
    document = {
        "schema_version": schema.RECEIPT_SCHEMA_VERSION,
        "environment": "macos-latest",
        "base": SHA_A,
        "head": SHA_B,
        "tree": SHA_T,
        "scope_identity": "0123456789abcdef",
        "completion": "complete",
        "host": {
            "os": "MacOS",
            "kernel": "Darwin 27.0.0",
            "report_dir": "~/ci-evidence/bbbbbbbbb",
        },
        "cells": [receipt_cell()],
    }
    document.update(overrides)
    return document


PROOF = {
    "nextest_version": "0.9.99",
    "target": "aarch64-apple-darwin",
    "expected_listing_digest": "e1" * 32,
    "completion_record_digest": "c0" * 32,
}


def body(kind):
    """An observation body of each producer kind, before identity."""
    document = {
        "producer": {
            "kind": "hook",
            "host": {"os": "MacOS", "kernel": "Darwin 27.0.0", "report_dir": "/evidence"},
        },
        "base": SHA_A,
        "scope_identity": "0123456789abcdef",
        "completion": "complete",
        "cell": receipt_cell(),
        "reports": [
            {"path": "claudine-L1.xml", "sha256": "ab" * 32, "bytes": 12, "store": "host-dir"}
        ],
    }
    if kind == "rig":
        document["producer"] = {
            "kind": "rig",
            "host": {
                "os": "Darwin",
                "kernel": "Darwin 27.0.0",
                "label": "mac-rig",
                "report_dir": "/evidence/rig/claudine-L1-run7",
            },
            "run_id": "run7",
        }
        document["proof"] = dict(PROOF)
    elif kind == "hosted":
        document["producer"] = {
            "kind": "hosted",
            "host": {"os": "Darwin", "kernel": "Darwin 24.6.0", "label": "macos-latest"},
            "workflow": ".github/workflows/ci.yml",
            "run_id": "100",
            "attempt": 1,
        }
        document["proof"] = dict(PROOF)
        document["cell"] = receipt_cell(report="L1/claudine.xml")
        document["reports"] = [
            {"path": "L1/claudine.xml", "sha256": "cd" * 32, "bytes": 420, "store": "artifact"}
        ]
    elif kind == "seed":
        document["producer"] = {
            "kind": "seed",
            "host": {
                "os": "Darwin",
                "kernel": "Darwin 27.0.0",
                "report_dir": "/evidence/seed/bbbbbbbbbbbb-0123456789ab",
            },
            "seed_id": "bbbbbbbbbbbb-0123456789ab",
        }
        document["proof"] = dict(PROOF)
    return document


def identified(document):
    return observation_notes.with_identity(document)


def collection(*observations, environment="macos-latest", **overrides):
    document = {
        "schema_version": schema.COLLECTION_SCHEMA_VERSION,
        "environment": environment,
        "head": SHA_B,
        "tree": SHA_T,
        "observations": sorted(observations, key=lambda o: str(o.get("observation_id"))),
    }
    document.update(overrides)
    return document


def set_path(document, path, value):
    target = document
    for step in path[:-1]:
        target = target[step]
    if value is DELETE:
        del target[path[-1]]
    else:
        target[path[-1]] = value


# ---------------------------------------------------------------------------
# canonical: raw text -> loads_strict -> canonical -> identity
# ---------------------------------------------------------------------------

CANONICAL = [
    ("unicode-key-order", "Keys sort by code point: U+FB01 before U+1F600, which UTF-16 order reverses.",
     '{"\U0001F600":1,"\ufb01":2,"z":3,"\u00e9":4,"Z":5,"":6,"a":7}'),
    ("unicode-key-order-escaped", "The same keys written as escapes, including a surrogate pair.",
     r'{"\ud83d\ude00":1,"\uFB01":2,"\u00E9":4}'),
    ("unicode-key-order-surrogates", "Lone surrogate, U+E000 and U+10000 keys: code point order D800 < E000 < 10000.",
     r'{"\ud800\udc00":3,"\ue000":2,"\ud800":1}'),
    ("non-ascii-values", "Non-ASCII values are \\u-escaped in lowercase hex; astral characters become surrogate pairs.",
     '{"v":"caf\u00e9 \u65e5\u672c \U0001F600 \u2028 \u00ff"}'),
    ("lone-surrogates", "Lone surrogate escapes survive into the canonical text.",
     r'{"v":"\ud800","w":"\udc00x","x":"\uDBFF"}'),
    ("control-characters", "Control characters: short escapes for \\b\\f\\n\\r\\t, \\u00XX otherwise; DEL is escaped too.",
     '{"v":"\\u0000\\u0001\\b\\f\\n\\r\\t\\u001f\\u0020\x7f"}'),
    ("slash-and-quote", "Forward slash is not escaped; an escaped slash loses its escape; quote and backslash are escaped.",
     r'{"v":"a/b\/c\"d\\e"}'),
    ("large-integers", "Integers keep every digit: beyond 2^53, beyond 2^64, and negative zero becomes 0.",
     '{"a":9007199254740993,"b":18446744073709551617,"c":-9223372036854775809,"d":-0,"e":0,'
     '"f":123456789012345678901234567890,"g":-1}'),
    ("floats", "Float tokens re-serialize through Python float repr.",
     '{"a":1.0,"b":1e5,"c":1E+5,"d":1.5e-7,"e":0.1,"f":-0.0,"g":100000000000000000000.0,'
     '"h":1e16,"i":1e15,"j":5e-324,"k":1.7976931348623157e308,"l":0.000001,"m":0.0001,'
     '"n":2.5E-3,"o":1e-5,"p":123.456e2,"q":-1.0e0,"r":1e22,"s":9007199254740993.0,'
     '"t":0.30000000000000004,"u":1e-7,"v":12345678901234567.0,"w":0E0,"x":-0e-0}'),
    ("float-overflow", "1e400 parses to inf; canonical emits the non-JSON token Infinity.",
     '{"a":1e400,"b":-1e400}'),
    ("float-underflow", "1e-400 underflows to 0.0.",
     '{"a":1e-400,"b":-1e-400}'),
    ("structure", "Nested containers, literals, empty objects and arrays, insignificant whitespace.",
     '[ 1 , { "b" : [ ] , "a" : { } } , "x" , null , true , false , [ [ ] ] ]\n'),
    ("top-level-string", "A scalar top-level document canonicalizes too.",
     r'"\u00e9"'),
]

for name, note, text in CANONICAL:
    emit(f"{name}.json", text)
    canonical = schema.canonical(schema.loads_strict(text))
    emit(f"{name}.canonical", canonical.encode("ascii"))
    CASES.append({
        "name": name,
        "kind": "canonical",
        "note": note,
        "input": f"{name}.json",
        "canonical": f"{name}.canonical",
        "identity": schema.identity(canonical),
        "files": [f"{name}.json", f"{name}.canonical"],
    })


# ---------------------------------------------------------------------------
# loader: bytes -> strict UTF-8 decode -> loads_strict
# ---------------------------------------------------------------------------

LOADER = [
    ("loader-duplicate-key", "Duplicate key, even with equal values.", b'{"a":1,"a":1}'),
    ("loader-duplicate-key-nested", "Duplicate key inside a nested object.", b'{"x":{"a":1,"a":2}}'),
    ("loader-trailing-garbage", "Non-whitespace after the document.", b'{"a":1} x'),
    ("loader-concatenated", "A second document directly after the first.", b'{"a":1}{"b":2}'),
    ("loader-concatenated-newline", "A second document on the next line (JSON Lines).", b'{"a":1}\n{"b":2}\n'),
    ("loader-nan", "NaN constant.", b'{"a":NaN}'),
    ("loader-infinity", "Infinity constant.", b'{"a":Infinity}'),
    ("loader-negative-infinity", "-Infinity constant.", b'{"a":-Infinity}'),
    ("loader-nan-nested", "NaN inside an array.", b'{"a":[1,NaN]}'),
    ("loader-bom", "UTF-8 byte order mark before the document.", b'\xef\xbb\xbf{"a":1}'),
    ("loader-blank", "Empty input.", b''),
    ("loader-whitespace-only", "Whitespace-only input.", b' \n\t\r\n'),
    ("loader-invalid-utf8", "A lone 0xFF byte inside a string.", b'{"a":"\xff"}'),
    ("loader-overlong-utf8", "Overlong two-byte encoding of '/'.", b'{"a":"\xc0\xaf"}'),
    ("loader-encoded-surrogate", "UTF-8-encoded surrogate U+D800 (CESU style).", b'{"a":"\xed\xa0\x80"}'),
    ("loader-truncated-utf8", "Truncated three-byte sequence.", b'{"a":"\xe6\x97"}'),
    ("loader-trailing-whitespace", "Whitespace after the document is accepted.", b'{"a":1}\n  \t\r\n'),
    ("loader-leading-whitespace", "Whitespace before the document is accepted.", b'\n\t {"a":1}'),
    ("loader-single-quotes", "JSON5-style single quotes.", b"{'a':1}"),
    ("loader-trailing-comma", "Trailing comma.", b'{"a":1,}'),
    ("loader-comment", "A comment.", b'{"a":1 /* c */}'),
    ("loader-leading-zero", "Leading zero.", b'{"a":01}'),
    ("loader-plus-sign", "Leading plus.", b'{"a":+1}'),
    ("loader-bare-fraction", "Fraction without integer part.", b'{"a":.5}'),
    ("loader-dangling-point", "Decimal point without digits.", b'{"a":1.}'),
    ("loader-raw-control", "A raw control character inside a string.", b'{"a":"\x01"}'),
    ("loader-raw-del", "A raw DEL byte inside a string is accepted.", b'{"a":"\x7f"}'),
    ("loader-raw-line-separator", "A raw U+2028 inside a string is accepted.", b'{"a":"\xe2\x80\xa8"}'),
    ("loader-invalid-escape", "Unknown escape sequence.", b'{"a":"\\x41"}'),
    ("loader-lone-surrogate-escape", "A lone surrogate escape is accepted.", b'{"a":"\\ud800"}'),
    ("loader-nul-escape", "An escaped NUL is accepted.", b'{"a":"\\u0000"}'),
    ("loader-top-level-scalar", "A scalar top-level document is accepted by the loader.", b'1'),
    ("loader-yaml", "YAML is not JSON.", b'a: 1\n'),
    ("loader-unterminated", "Unterminated object.", b'{'),
    ("loader-uppercase-literal", "Literals are case sensitive.", b'{"a":True}'),
]

for name, note, data in LOADER:
    record = {"name": name, "kind": "loader", "note": note}
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        record.update(accepts=False, stage="decode", exception=type(error).__name__, error=str(error))
    else:
        try:
            document = schema.loads_strict(text)
        except ValueError as error:
            record.update(accepts=False, stage="parse", exception=type(error).__name__, error=str(error))
        else:
            record.update(accepts=True, canonical_text=schema.canonical(document))
    extension = "json" if record["accepts"] else "txt"
    emit(f"{name}.{extension}", data)
    record["input"] = f"{name}.{extension}"
    record["files"] = [record["input"]]
    CASES.append(record)


# ---------------------------------------------------------------------------
# collection: real-shape v3 collections from the real producers
# ---------------------------------------------------------------------------


def collection_case(name, note, document, source):
    emit(f"{name}.json", pretty(document))
    result, text = collection_record(document)
    emit(f"{name}.canonical", text.encode("ascii"))
    CASES.append({
        "name": name,
        "kind": "collection",
        "note": note,
        "source": source,
        "input": f"{name}.json",
        "canonical": f"{name}.canonical",
        "files": [f"{name}.json", f"{name}.canonical"],
        **result,
    })


HOME = SCRATCH / "home"


def use_home(path):
    path.mkdir(parents=True, exist_ok=True)
    os.environ["HOME"] = str(path)
    os.environ["USERPROFILE"] = str(path)


def junit(package, passed, failures=()):
    cases = "".join(
        f'<testcase classname="{package}" name="{case}"><failure message="boom"/></testcase>'
        for case in failures
    ) + "".join(f'<testcase classname="{package}" name="ok_{i}"/>' for i in range(passed))
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n<testsuites>'
        f'<testsuite name="{package}" tests="{passed + len(failures)}" failures="{len(failures)}" '
        f'errors="0" skipped="0">{cases}</testsuite></testsuites>\n'
    )


# Rig: the tail of observation_notes.record_rig_cell, which hands its receipt to
# receipt_observations(kind="rig", proof=..., features=..., run_id=...). The proof
# digests are the real SHA-256 of the documents it would retain.
use_home(HOME / "rig")
rig_dir = "~/ci-evidence/bbbbbbbbb/macos-latest/biscuit-hash-L1-run7"
rig_root = HOME / "rig" / "ci-evidence/bbbbbbbbb/macos-latest/biscuit-hash-L1-run7"
rig_root.mkdir(parents=True)
(rig_root / "biscuit-hash-L1.xml").write_text(junit("biscuit-hash", 4), encoding="utf-8")
expected_doc = rig_root / schema.PROOF_EXPECTED_FILE.format(package="biscuit-hash", gate="L1")
expected_doc.write_text('{"target":"aarch64-apple-darwin","nextest_version":"0.9.99"}\n', encoding="utf-8")
completion_doc = rig_root / schema.PROOF_COMPLETION_FILE.format(package="biscuit-hash", gate="L1")
completion_doc.write_text('{"complete":true}\n', encoding="utf-8")
rig_receipt = receipt(
    host={"os": "Darwin", "kernel": "Darwin 27.0.0", "report_dir": rig_dir},
    cells=[receipt_cell(
        package="biscuit-hash", report="biscuit-hash-L1.xml",
        counts={"total": 4, "passed": 4, "failed": 0, "skipped": 0, "errored": 0},
    )],
)
rig_observations = observation_notes.receipt_observations(
    rig_receipt,
    kind="rig",
    proof={
        "nextest_version": "0.9.99",
        "target": "aarch64-apple-darwin",
        "expected_listing_digest": observation_notes._sha256(expected_doc),
        "completion_record_digest": observation_notes._sha256(completion_doc),
    },
    features="--features x",
    run_id="run7",
)
collection_case(
    "collection-rig",
    "A passing rig observation with full proof (completion_record_digest) and features.",
    observation_notes.collection_of("macos-latest", SHA_B, SHA_T, rig_observations),
    "observation_notes.receipt_observations(kind='rig', proof, features, run_id) + collection_of",
)

# Seed: no producer writes seed_id today, so the rig path is reused and the
# producer gains seed_id before identity is computed.
use_home(HOME / "seed")
seed_root = HOME / "seed" / "ci-evidence/seed/bbbbbbbbbbbb-0123456789ab"
seed_root.mkdir(parents=True)
(seed_root / "claudine-L1.xml").write_text(junit("claudine", 9), encoding="utf-8")
seed_observations = observation_notes.receipt_observations(
    receipt(host={"os": "Darwin", "kernel": "Darwin 27.0.0",
                  "report_dir": "~/ci-evidence/seed/bbbbbbbbbbbb-0123456789ab"}),
    kind="seed",
    proof=dict(PROOF),
    run_id="seed-run-1",
)
seed_observations = [
    identified({**o, "producer": {**o["producer"], "seed_id": "bbbbbbbbbbbb-0123456789ab"}})
    for o in seed_observations
]
collection_case(
    "collection-seed",
    "A passing seed observation carrying seed_id.",
    observation_notes.collection_of("macos-latest", SHA_B, SHA_T, seed_observations),
    "observation_notes.receipt_observations(kind='seed') with producer.seed_id added, re-identified",
)

# Hosted: the real hosted_observations.build_observation over a staged cell
# shaped by scripts/ci/test_hosted_observations.py. The checkout probe is the
# only replaced function: it would otherwise ask Git for HEAD.
local_evidence.revision = lambda reference: SHA_B
fixture = types.SimpleNamespace(first=SHA_A, second=SHA_B, scratch=SCRATCH / "hosted", staged=0)
fixture.scratch.mkdir(parents=True)
hosted_plan = test_hosted_observations.HostedFixture.plan(fixture)
run = hosted_observations.RunContext(
    run_id="100", attempt=1, workflow=".github/workflows/ci.yml",
    host_os="Linux", host_kernel="Linux 6.8.0-1017-azure",
)
for outcome, failing in (("pass", ()), ("fail", ("broken_case",))):
    stage, completion_path = test_hosted_observations.HostedFixture.stage_cell(
        fixture, "alpha", "ubuntu-latest", failing=failing, completion=not failing,
    )
    envelope = hosted_observations.build_observation(
        hosted_plan, "alpha/ubuntu-latest/L1", stage,
        completion_path if not failing else None,
        "failure" if failing else "success", run,
    )
    collection_case(
        f"collection-hosted-{outcome}",
        f"A hosted {outcome} observation (artifact report store, workflow/run_id/attempt).",
        observation_notes.collection_of(
            envelope["environment"], envelope["head"], SHA_T, [envelope["observation"]]
        ),
        "hosted_observations.build_observation + observation_notes.collection_of",
    )

# Incomplete: an interrupted cell cites no report.
incomplete = identified({
    **body("hook"),
    "completion": "interrupted",
    "cell": receipt_cell(
        outcome="fail", completion="interrupted", exit_code=101,
        counts={"total": 0, "passed": 0, "failed": 0, "skipped": 0, "errored": 0},
    ),
    "reports": [],
})
collection_case(
    "collection-incomplete",
    "An interrupted hook observation with no reports and no failed tests.",
    collection(incomplete),
    "with_identity over a test_schema.observation-shaped body",
)

# Retry chain: interrupted -> complete fail -> pass, one cell.
first = identified({
    **body("rig"),
    "completion": "interrupted",
    "cell": receipt_cell(
        outcome="fail", completion="interrupted", exit_code=101,
        counts={"total": 0, "passed": 0, "failed": 0, "skipped": 0, "errored": 0},
    ),
    "reports": [],
})
del first["proof"]
first = identified(first)
second = observation_notes.with_retry_of(identified({
    **body("rig"),
    "cell": receipt_cell(
        outcome="fail", exit_code=100, failed_tests=["claudine::tests::flaky"],
        counts={"total": 10, "passed": 8, "failed": 1, "skipped": 1, "errored": 0},
    ),
    "proof": {k: v for k, v in PROOF.items() if k != "completion_record_digest"},
}), first["observation_id"])
third = observation_notes.with_retry_of(identified(body("rig")), second["observation_id"])
collection_case(
    "collection-retry-chain",
    "Three rig observations of one cell linked by retry_of.",
    collection(first, second, third),
    "observation_notes.with_retry_of",
)

feature_variants = [identified(body("hook"))]
for value in ("", "--features x"):
    feature_variants.append(identified({**body("hook"), "features": value}))
collection_case(
    "collection-features-variants",
    "Identical hook observations whose features are absent, empty, and '--features x': three distinct ids.",
    collection(*feature_variants),
    "with_identity",
)

collection_case(
    "collection-empty",
    "A real empty observations list is valid zero proof.",
    collection(),
    "test_schema.collection()",
)

collection_case(
    "collection-unicode",
    "Non-ASCII report path, host label and failed test name.",
    collection(identified({
        **body("rig"),
        "producer": {**body("rig")["producer"],
                     "host": {**body("rig")["producer"]["host"], "label": "r\u00e9g\u00eel \U0001F600"}},
        "cell": receipt_cell(
            outcome="fail", exit_code=100, report="caf\u00e9-L1.xml",
            failed_tests=["caf\u00e9::\u65e5\u672c::\U0001F600"],
            counts={"total": 10, "passed": 8, "failed": 1, "skipped": 1, "errored": 0},
        ),
        "reports": [{"path": "caf\u00e9-L1.xml", "sha256": "ab" * 32, "bytes": 12, "store": "host-dir"}],
    })),
    "with_identity",
)


# ---------------------------------------------------------------------------
# receipt-adapter: v2 receipt -> receipt_observations (hook default) -> collection_of
# ---------------------------------------------------------------------------


def adapter_case(name, note, document, reports):
    """reports maps a path under HOME to its bytes; HOME is <name>.home."""
    home = HOME / name
    use_home(home)
    files = [f"{name}.json"]
    report_records = []
    for relative, content in sorted(reports.items()):
        (home / relative).parent.mkdir(parents=True, exist_ok=True)
        (home / relative).write_bytes(content)
        emit(f"{name}.home/{relative}", content)
        files.append(f"{name}.home/{relative}")
    emit(f"{name}.json", pretty(document))
    record = {
        "name": name,
        "kind": "receipt-adapter",
        "note": note,
        "input": f"{name}.json",
        "home": f"{name}.home",
        "receipt_problems": schema.validate_receipt(document),
    }
    try:
        observations = observation_notes.receipt_observations(json.loads(pretty(document)))
        adapted = observation_notes.collection_of(
            document["environment"], document["head"], document["tree"], observations
        )
    except observation_notes.PublicationError as error:
        record["error"] = str(error).replace(str(home), "$HOME")
    else:
        result, text = collection_record(adapted)
        emit(f"{name}.canonical", text.encode("ascii"))
        files.append(f"{name}.canonical")
        record.update(canonical=f"{name}.canonical", **result)
    record["files"] = sorted(files)
    CASES.append(record)


adapter_case(
    "adapter-hook-v2",
    "Three cells: complete pass, complete fail with detail, interrupted (no report read).",
    receipt(
        completion="partial",
        cells=[
            receipt_cell(),
            receipt_cell(
                package="darkmatter", outcome="fail", exit_code=100, report="darkmatter-L1.xml",
                failed_tests=["darkmatter::render::tables"],
                counts={"total": 3, "passed": 2, "failed": 1, "skipped": 0, "errored": 0},
            ),
            receipt_cell(
                package="biscuit-hash", gate="L2", outcome="fail", exit_code=101,
                completion="interrupted", report="biscuit-hash-L2.xml", backends=["tmux"],
                counts={"total": 0, "passed": 0, "failed": 0, "skipped": 0, "errored": 0},
            ),
        ],
    ),
    {
        "ci-evidence/bbbbbbbbb/claudine-L1.xml": junit("claudine", 9).encode("utf-8"),
        "ci-evidence/bbbbbbbbb/darkmatter-L1.xml": junit("darkmatter", 2, ("render::tables",)).encode("utf-8"),
    },
)
adapter_case(
    "adapter-hook-v2-bare-home",
    "report_dir is '~' itself; the report path has a subdirectory and a space, and the report is empty.",
    receipt(
        environment="ubuntu-latest",
        host={"os": "Linux", "kernel": "Linux 6.8.0", "report_dir": "~"},
        cells=[receipt_cell(report="nested/claudine L1.xml")],
    ),
    {"nested/claudine L1.xml": b""},
)
adapter_case(
    "adapter-v2-missing-report",
    "A complete cell whose report is not retained: the adapter refuses.",
    receipt(),
    {},
)
adapter_case(
    "adapter-v2-empty-cells",
    "cells: [] is invalid for a v2 receipt.",
    receipt(cells=[]),
    {},
)
adapter_case(
    "adapter-v1-receipt",
    "A version-1 receipt has no adapter.",
    {"schema_version": 1, "environment": "macos-latest", "head": SHA_B, "tree": SHA_T},
    {},
)
adapter_case(
    "adapter-v2-identical-cells",
    "Two identical cells pass validate_receipt but adapt to one id twice; collection_of refuses.",
    receipt(cells=[receipt_cell(), receipt_cell()]),
    {"ci-evidence/bbbbbbbbb/claudine-L1.xml": junit("claudine", 9).encode("utf-8")},
)


# ---------------------------------------------------------------------------
# validator: one edit per case from an authentic base
# ---------------------------------------------------------------------------


def validator_case(name, target, document, note=""):
    text = schema.canonical(document)
    emit(f"{name}.json", text)
    reparsed = schema.loads_strict(text)
    record = {"name": name, "kind": "validator", "target": target, "input": f"{name}.json",
              "files": [f"{name}.json"]}
    if note:
        record["note"] = note
    try:
        problems = (schema.validate_collection if target == "collection" else schema.validate_receipt)(reparsed)
    except Exception as error:  # noqa: BLE001 - a crash is part of the contract to record
        record.update(problems=None, codes=None, exception=f"{type(error).__name__}: {error}")
    else:
        record.update(problems=problems, codes=codes(problems), valid=not problems)
    CASES.append(record)


def obs_case(name, kind, edits, rehash=True, note=""):
    """One observation of kind, edited at edits [(path, value)], in a collection."""
    observation = identified(body(kind))
    for path, value in edits:
        set_path(observation, path, value)
    if rehash:
        observation = identified(observation)
    validator_case(name, "collection", collection(observation), note)


def coll_case(name, edits, note=""):
    document = collection(identified(body("hook")))
    for path, value in edits:
        set_path(document, path, value)
    validator_case(name, "collection", document, note)


def receipt_case(name, edits, note=""):
    document = receipt()
    for path, value in edits:
        set_path(document, path, value)
    validator_case(name, "receipt", document, note)


# Controls.
for kind in ("hook", "rig", "hosted", "seed"):
    obs_case(f"v-control-{kind}", kind, [], note="Unedited authentic observation: valid.")
receipt_case("v-receipt-control", [], "Unedited authentic receipt: valid.")

# Strict evidence object/version: collection.
validator_case("v-collection-array", "collection", [], "Top-level array.")
validator_case("v-collection-null", "collection", None, "Top-level null.")
validator_case("v-collection-empty-object", "collection", {}, "Empty object: no schema_version, every field missing.")
coll_case("v-collection-unknown-key", [(("extra",), 1)])
coll_case("v-collection-version-float", [(("schema_version",), 3.0)])
coll_case("v-collection-version-true", [(("schema_version",), True)])
coll_case("v-collection-version-string", [(("schema_version",), "3")])
coll_case("v-collection-version-null", [(("schema_version",), None)])
coll_case("v-collection-version-2", [(("schema_version",), 2)], "A v2 receipt version is not a collection.")
coll_case("v-collection-missing-environment", [(("environment",), DELETE)])
coll_case("v-collection-missing-observations", [(("observations",), DELETE)])
coll_case("v-collection-environment-unknown", [(("environment",), "linux")])
coll_case("v-collection-environment-int", [(("environment",), 1)])
coll_case("v-collection-head-short", [(("head",), "b" * 39)])
coll_case("v-collection-head-uppercase", [(("head",), "B" * 40)])
coll_case("v-collection-tree-null", [(("tree",), None)])
coll_case("v-collection-observations-null", [(("observations",), None)])
coll_case("v-collection-observations-object", [(("observations",), {})])
coll_case("v-collection-observation-null", [(("observations",), [None])])
coll_case("v-collection-observation-scalar", [(("observations",), [1])])
coll_case("v-collection-observations-all-invalid", [(("observations",), [None, 1, "x"])])
coll_case("v-collection-observation-empty-object", [(("observations",), [{}])])
coll_case(
    "v-collection-environment-cell-label", [(("environment",), "ubuntu-latest")],
    "The collection environment only labels its cells; an observation is not checked against it.",
)

# Strict object: observation, producer, host, proof, report, cell, counts, build.
obs_case("v-observation-unknown-key", "hook", [(("extra",), 1)])
obs_case("v-observation-missing-base", "hook", [(("base",), DELETE)])
obs_case("v-observation-missing-id", "hook", [(("observation_id",), DELETE)], rehash=False)
obs_case("v-producer-null", "hook", [(("producer",), None)])
obs_case("v-producer-string", "hook", [(("producer",), "hook")])
obs_case("v-producer-empty-object", "hook", [(("producer",), {})])
obs_case("v-producer-unknown-key", "hook", [(("producer", "extra"), "x")])
obs_case("v-host-null", "hook", [(("producer", "host"), None)])
obs_case("v-host-empty-object", "hook", [(("producer", "host"), {})])
obs_case("v-host-unknown-key", "hook", [(("producer", "host", "extra"), "x")])
obs_case("v-proof-null", "rig", [(("proof",), None)])
obs_case("v-proof-empty-object", "rig", [(("proof",), {})])
obs_case("v-proof-unknown-key", "rig", [(("proof", "extra"), "x")])
obs_case("v-report-null", "hook", [(("reports",), [None])])
obs_case("v-report-empty-object", "hook", [(("reports",), [{}])])
obs_case("v-report-unknown-key", "hook", [(("reports", 0, "extra"), "x")])
obs_case("v-cell-null", "hook", [(("cell",), None)])
obs_case("v-cell-empty-object", "hook", [(("cell",), {})])
obs_case("v-cell-unknown-key", "hook", [(("cell", "extra"), "x")])
obs_case("v-cell-missing-package", "hook", [(("cell", "package"), DELETE)])
obs_case("v-counts-null", "hook", [(("cell", "counts"), None)])
obs_case("v-counts-list", "hook", [(("cell", "counts"), [])])
obs_case("v-counts-empty-object", "hook", [(("cell", "counts"), {})])
obs_case("v-counts-missing-total", "hook", [(("cell", "counts", "total"), DELETE)])
obs_case("v-counts-unknown-key", "hook", [(("cell", "counts", "extra"), 1)],
         note="QUIRK: _counts never rejects unknown keys.")
obs_case("v-build-null", "hook", [(("cell", "build"), None)])
obs_case("v-build-valid", "hook", [(("cell", "build"), {"key": "0123456789abcdef", "digest": "ab" * 32})])
obs_case("v-build-unknown-key", "hook", [(("cell", "build"), {"key": "0123456789abcdef", "digest": "ab" * 32, "x": 1})])
obs_case("v-build-key-not-hex", "hook", [(("cell", "build"), {"key": "not-hex!", "digest": "ab" * 32})])
obs_case("v-build-key-int-digits", "hook", [(("cell", "build"), {"key": 1234567890123456, "digest": "ab" * 32})],
         note="QUIRK: str() coercion lets an integer whose decimal digits match [0-9a-f]{8,64} pass.")

# String/enum/identity.
obs_case("v-observation-id-not-hex", "hook", [(("observation_id",), "zz" * 32)], rehash=False)
obs_case("v-observation-id-uppercase", "hook", [(("observation_id",), "AB" * 32)], rehash=False)
obs_case("v-observation-id-int", "hook", [(("observation_id",), 12345678)], rehash=False)
obs_case("v-observation-id-mismatch", "hook", [(("base",), "d" * 40)], rehash=False)
obs_case("v-retry-of-null", "hook", [(("retry_of",), None)])
obs_case("v-retry-of-not-hex", "hook", [(("retry_of",), "not-an-id")])
obs_case("v-retry-of-int", "hook", [(("retry_of",), 12345678)])
obs_case("v-base-short", "hook", [(("base",), "a" * 12)])
obs_case("v-base-null", "hook", [(("base",), None)])
obs_case("v-scope-identity-short", "hook", [(("scope_identity",), "abc")])
obs_case("v-scope-identity-uppercase", "hook", [(("scope_identity",), "ABCDEF0123")])
obs_case("v-scope-identity-int-digits", "hook", [(("scope_identity",), 12345678)],
         note="QUIRK: str(12345678) matches [0-9a-f]{8,64}, so an integer passes.")
obs_case("v-scope-identity-float", "hook", [(("scope_identity",), 12345678.0)])
obs_case("v-scope-identity-true", "hook", [(("scope_identity",), True)])
obs_case("v-scope-identity-null", "hook", [(("scope_identity",), None)])
obs_case("v-completion-unknown", "hook", [(("completion",), "done")])
obs_case("v-completion-null", "hook", [(("completion",), None)])
obs_case("v-features-empty", "hook", [(("features",), "")], note="Empty features stays a valid string.")
obs_case("v-features-value", "hook", [(("features",), "--features x")])
obs_case("v-features-null", "hook", [(("features",), None)])
obs_case("v-features-list", "hook", [(("features",), ["x"])])
obs_case("v-producer-kind-unknown", "hook", [(("producer", "kind"), "robot")])
obs_case("v-producer-kind-null", "hook", [(("producer", "kind"), None)])
obs_case("v-host-os-empty", "hook", [(("producer", "host", "os"), "")])
obs_case("v-host-kernel-int", "hook", [(("producer", "host", "kernel"), 6)])
obs_case("v-host-label-int", "rig", [(("producer", "host", "label"), 1)])
obs_case("v-host-report-dir-int", "hook", [(("producer", "host", "report_dir"), 1)])
obs_case("v-host-report-dir-empty", "hook", [(("producer", "host", "report_dir"), "")],
         note="host-dir report with an empty report_dir.")
obs_case("v-host-report-dir-absent", "hook", [(("producer", "host", "report_dir"), DELETE)])
obs_case("v-hosted-artifact-no-report-dir", "hosted", [],
         note="An artifact report needs no report_dir.")
obs_case("v-hosted-missing-workflow", "hosted", [(("producer", "workflow"), DELETE)])
obs_case("v-hosted-missing-run-id", "hosted", [(("producer", "run_id"), DELETE)])
obs_case("v-hosted-missing-attempt", "hosted", [(("producer", "attempt"), DELETE)])
obs_case("v-hosted-workflow-int", "hosted", [(("producer", "workflow"), 1)])
obs_case("v-hosted-run-id-int", "hosted", [(("producer", "run_id"), 100)])
obs_case("v-hosted-attempt-zero", "hosted", [(("producer", "attempt"), 0)])
obs_case("v-hosted-attempt-negative", "hosted", [(("producer", "attempt"), -1)])
obs_case("v-hosted-attempt-float", "hosted", [(("producer", "attempt"), 1.0)])
obs_case("v-hosted-attempt-true", "hosted", [(("producer", "attempt"), True)])
obs_case("v-hosted-attempt-string", "hosted", [(("producer", "attempt"), "1")])
obs_case("v-hook-with-run-id", "hook", [(("producer", "run_id"), "r1")],
         note="Optional producer fields are allowed on any kind.")
obs_case("v-seed-missing-seed-id", "seed", [(("producer", "seed_id"), DELETE)])
obs_case("v-seed-seed-id-int", "seed", [(("producer", "seed_id"), 1)])
obs_case("v-proof-digest-not-hex", "rig", [(("proof", "expected_listing_digest"), "not hex at all"),
                                           (("proof", "completion_record_digest"), "sha256:XYZ")],
         note="VALID: proof digests are any nonempty string.")
obs_case("v-proof-digest-empty", "rig", [(("proof", "expected_listing_digest"), "")])
obs_case("v-proof-target-int", "rig", [(("proof", "target"), 1)])
obs_case("v-proof-missing-target", "rig", [(("proof", "target"), DELETE)])
obs_case("v-report-path-empty", "hook", [(("reports", 0, "path"), "")])
obs_case("v-report-path-int", "hook", [(("reports", 0, "path"), 1)])
obs_case("v-report-sha256-not-hex", "hook", [(("reports", 0, "sha256"), "zz" * 32)])
obs_case("v-report-sha256-uppercase", "hook", [(("reports", 0, "sha256"), "AB" * 32)])
obs_case("v-report-sha256-short", "hook", [(("reports", 0, "sha256"), "ab" * 31)])
obs_case("v-report-store-unknown", "hook", [(("reports", 0, "store"), "r2")])
obs_case("v-report-store-null", "hook", [(("reports", 0, "store"), None)])
obs_case("v-report-bytes-negative", "hook", [(("reports", 0, "bytes"), -1)])
obs_case("v-report-bytes-zero", "hook", [(("reports", 0, "bytes"), 0)])
obs_case("v-report-bytes-float", "hook", [(("reports", 0, "bytes"), 12.0)])
obs_case("v-report-bytes-true", "hook", [(("reports", 0, "bytes"), True)])
obs_case("v-report-bytes-string", "hook", [(("reports", 0, "bytes"), "12")])
obs_case("v-cell-package-empty", "hook", [(("cell", "package"), "")])
obs_case("v-cell-package-int", "hook", [(("cell", "package"), 1)])
obs_case("v-cell-gate-int", "hook", [(("cell", "gate"), 1)])
obs_case("v-cell-gate-unknown", "hook", [(("cell", "gate"), "L3")])
obs_case("v-cell-outcome-unknown", "hook", [(("cell", "outcome"), "ok")])
obs_case("v-cell-completion-unknown", "hook", [(("cell", "completion"), "done")])
obs_case("v-cell-gate-input-identity-null", "hook", [(("cell", "gate_input_identity"), None)])
obs_case("v-cell-gate-input-identity-int-digits", "hook", [(("cell", "gate_input_identity"), 1234567890)],
         note="QUIRK: str() coercion lets an integer of decimal digits pass.")
obs_case("v-cell-report-int", "hook", [(("cell", "report"), 1)])
obs_case("v-cell-test-filter-l1", "hook", [(("cell", "test_filter"), "test(one)")])
obs_case("v-cell-test-filter-blank", "hook", [(("cell", "test_filter"), "  ")])
obs_case("v-cell-test-filter-l2", "hook", [(("cell", "gate"), "L2"), (("cell", "test_filter"), "test(one)")])

# Lists and members.
obs_case("v-reports-null", "hook", [(("reports",), None)])
obs_case("v-reports-string", "hook", [(("reports",), "claudine-L1.xml")])
obs_case("v-reports-empty-complete", "hook", [(("reports",), [])],
         note="A complete cell whose report no reference cites.")
obs_case("v-reports-all-invalid", "hook", [(("reports",), [None, 1, "x"])])
obs_case("v-reports-mixed", "hook", [(("reports",), [body("hook")["reports"][0], None])])
obs_case("v-backends-null", "hook", [(("cell", "backends"), None)])
obs_case("v-backends-string", "hook", [(("cell", "backends"), "tmux")])
obs_case("v-backends-int-element", "hook", [(("cell", "backends"), ["tmux", 1])])
obs_case("v-backends-all-invalid", "hook", [(("cell", "backends"), [1, None])])
obs_case("v-backends-missing", "hook", [(("cell", "backends"), DELETE)])
obs_case("v-failed-tests-null", "hook", [(("cell", "failed_tests"), None)],
         note="Present-but-null is not the absent default.")
obs_case("v-failed-tests-empty", "hook", [(("cell", "failed_tests"), [])])
obs_case("v-failed-tests-int-element", "hook", [(("cell", "failed_tests"), ["a", 1])])
FAIL_COUNTS = {"total": 21, "passed": 0, "failed": 21, "skipped": 0, "errored": 0}
obs_case("v-failed-tests-20", "hook", [(("cell", "outcome"), "fail"), (("cell", "exit_code"), 100),
                                       (("cell", "counts"), FAIL_COUNTS),
                                       (("cell", "failed_tests"), [f"t{i:02d}" for i in range(20)])])
obs_case("v-failed-tests-21", "hook", [(("cell", "outcome"), "fail"), (("cell", "exit_code"), 100),
                                       (("cell", "counts"), FAIL_COUNTS),
                                       (("cell", "failed_tests"), [f"t{i:02d}" for i in range(21)])])
obs_case("v-complete-fail-no-detail", "hook", [(("cell", "outcome"), "fail"), (("cell", "exit_code"), 100)])
obs_case("v-complete-fail-truncated", "hook", [(("cell", "outcome"), "fail"), (("cell", "exit_code"), 100),
                                               (("cell", "failure_detail_truncated"), True)])
obs_case("v-complete-fail-truncated-false", "hook", [(("cell", "outcome"), "fail"), (("cell", "exit_code"), 100),
                                                     (("cell", "failure_detail_truncated"), False)])
obs_case("v-complete-fail-truncated-string", "hook", [(("cell", "outcome"), "fail"), (("cell", "exit_code"), 100),
                                                      (("cell", "failure_detail_truncated"), "yes")],
         note="QUIRK: failure_detail_truncated is never type-checked; any truthy value excuses detail.")
obs_case("v-truncated-on-pass", "hook", [(("cell", "failure_detail_truncated"), 0)],
         note="QUIRK: failure_detail_truncated of any type is accepted on a pass.")
obs_case("v-interrupted-fail-no-detail", "hook", [(("cell", "outcome"), "fail"), (("cell", "exit_code"), 101),
                                                  (("cell", "completion"), "interrupted")])
obs_case("v-partial-no-reports", "hook", [(("cell", "completion"), "partial"), (("reports",), [])])

# Numeric/boolean/counts.
obs_case("v-exit-code-float", "hook", [(("cell", "exit_code"), 0.0)])
obs_case("v-exit-code-true", "hook", [(("cell", "exit_code"), True)])
obs_case("v-exit-code-null", "hook", [(("cell", "exit_code"), None)])
obs_case("v-exit-code-negative", "hook", [(("cell", "exit_code"), -1)], note="Any integer exit code is valid.")
obs_case("v-duration-float", "hook", [(("cell", "duration_s"), 1.5)])
obs_case("v-duration-negative", "hook", [(("cell", "duration_s"), -1)])
obs_case("v-duration-true", "hook", [(("cell", "duration_s"), True)])
obs_case("v-duration-string", "hook", [(("cell", "duration_s"), "35")])
obs_case("v-duration-null", "hook", [(("cell", "duration_s"), None)])
obs_case("v-counts-negative", "hook", [(("cell", "counts"), {"total": 9, "passed": 10, "failed": -1, "skipped": 0, "errored": 0})])
obs_case("v-counts-sum-mismatch", "hook", [(("cell", "counts", "total"), 11)])
obs_case("v-counts-float", "hook", [(("cell", "counts"), {"total": 10.0, "passed": 9, "failed": 0, "skipped": 1.0, "errored": 0})],
         note="QUIRK: float counts are never type-checked; the sum still matches.")
obs_case("v-counts-bool", "hook", [(("cell", "counts"), {"total": 1, "passed": True, "failed": False, "skipped": 0, "errored": 0})],
         note="QUIRK: booleans are Python ints and sum as 1/0.")
obs_case("v-counts-total-string", "hook", [(("cell", "counts", "total"), "10")],
         note="A string total compares unequal to the integer sum.")
obs_case("v-counts-part-string", "hook", [(("cell", "counts", "passed"), "9")],
         note="QUIRK: a string part makes Python's sum raise TypeError; the validator crashes.")
obs_case("v-counts-part-null", "hook", [(("cell", "counts", "skipped"), None)],
         note="QUIRK: a null part makes Python's sum raise TypeError; the validator crashes.")

# Producer kind and proof.
obs_case("v-hook-with-proof", "hook", [(("proof",), dict(PROOF))])
obs_case("v-rig-without-proof", "rig", [(("proof",), DELETE)])
obs_case("v-rig-fail-without-proof", "rig", [(("proof",), DELETE), (("cell", "outcome"), "fail"),
                                             (("cell", "exit_code"), 100),
                                             (("cell", "failed_tests"), ["claudine::t"])],
         note="VALID: a failing rig observation needs no proof.")
obs_case("v-hosted-without-proof", "hosted", [(("proof",), DELETE)])
obs_case("v-seed-without-proof", "seed", [(("proof",), DELETE)])
obs_case("v-pass-without-completion-digest", "rig", [(("proof", "completion_record_digest"), DELETE)])
obs_case("v-fail-without-completion-digest", "rig", [(("proof", "completion_record_digest"), DELETE),
                                                     (("cell", "outcome"), "fail"), (("cell", "exit_code"), 100),
                                                     (("cell", "failed_tests"), ["claudine::t"])],
         note="VALID: only a pass must cite a completion record.")

# Graph/context.
hook = identified(body("hook"))
other = identified({**body("hook"), "cell": receipt_cell(package="darkmatter", report="darkmatter-L1.xml"),
                    "reports": [{"path": "darkmatter-L1.xml", "sha256": "ab" * 32, "bytes": 12, "store": "host-dir"}]})
validator_case("v-graph-unsorted", "collection",
               {**collection(hook, other), "observations": sorted([hook, other], key=lambda o: o["observation_id"], reverse=True)})
validator_case("v-graph-duplicate-id", "collection", {**collection(hook), "observations": [hook, hook]})
validator_case("v-graph-retry-absent", "collection",
               collection(observation_notes.with_retry_of(hook, "f" * 64)))
validator_case("v-graph-retry-across-cells", "collection",
               collection(hook, observation_notes.with_retry_of(other, hook["observation_id"])))
filtered = identified({**body("hook"), "cell": receipt_cell(test_filter="test(one)")})
validator_case("v-graph-retry-across-test-filter", "collection",
               collection(hook, observation_notes.with_retry_of(filtered, hook["observation_id"])),
               "A test_filter is part of the cell key.")
validator_case("v-graph-retry-same-cell", "collection",
               collection(hook, observation_notes.with_retry_of(identified({**body("hook"), "features": ""}),
                                                                hook["observation_id"])))
# A cycle needs two ids that each hash the other: unconstructible with content
# ids. The closest input links two observations to each other without
# re-identifying, which the per-observation id check rejects first.
cycle_a = copy.deepcopy(hook)
cycle_b = copy.deepcopy(other)
cycle_a["retry_of"] = cycle_b["observation_id"]
cycle_b["retry_of"] = cycle_a["observation_id"]
validator_case("v-graph-retry-cycle", "collection", collection(cycle_a, cycle_b),
               "Unconstructible as a valid cycle: the id check fires before the graph check.")
self_retry = copy.deepcopy(hook)
self_retry["retry_of"] = self_retry["observation_id"]
validator_case("v-graph-retry-self", "collection", collection(self_retry))
validator_case("v-graph-invalid-member-hides-graph", "collection",
               {**collection(hook, other), "observations": [other, {**hook, "base": "x"}]},
               "Any observation problem returns before the sort and retry checks.")

# Receipt-only validator wiring.
receipt_case("v-receipt-v1", [(("schema_version",), 1)])
receipt_case("v-receipt-version-float", [(("schema_version",), 2.0)])
receipt_case("v-receipt-version-true", [(("schema_version",), True)])
receipt_case("v-receipt-version-3", [(("schema_version",), 3)])
receipt_case("v-receipt-missing-version", [(("schema_version",), DELETE)])
validator_case("v-receipt-null", "receipt", None)
validator_case("v-receipt-empty-object", "receipt", {})
receipt_case("v-receipt-unknown-key", [(("extra",), 1)])
receipt_case("v-receipt-cells-empty", [(("cells",), [])])
receipt_case("v-receipt-cells-null", [(("cells",), None)])
receipt_case("v-receipt-cell-null", [(("cells",), [None])])
receipt_case("v-receipt-cells-all-invalid", [(("cells",), [None, 1])])
receipt_case("v-receipt-host-null", [(("host",), None)])
receipt_case("v-receipt-host-missing-report-dir", [(("host", "report_dir"), DELETE)])
receipt_case("v-receipt-host-unknown-key", [(("host", "label"), "x")],
             "A receipt host has no label field, unlike an observation host.")
receipt_case("v-receipt-host-os-int", [(("host", "os"), 1)],
             "QUIRK: receipt host fields are key-checked only, never type-checked.")
receipt_case("v-receipt-scope-identity-int-digits", [(("scope_identity",), 12345678)],
             "QUIRK: str() coercion lets an integer of decimal digits pass.")
receipt_case("v-receipt-environment-unknown", [(("environment",), "linux")])
receipt_case("v-receipt-completion-unknown", [(("completion",), "done")])
receipt_case("v-receipt-tree-short", [(("tree",), "c" * 7)])
receipt_case("v-receipt-cells-identical", [(("cells",), [receipt_cell(), receipt_cell()])],
             "VALID: identical duplicate cells are not a conflict.")
receipt_case("v-receipt-cells-conflicting", [(("cells",), [receipt_cell(), receipt_cell(duration_s=36)])])


# ---------------------------------------------------------------------------

CASES.sort(key=lambda case: case["name"])
names = [case["name"] for case in CASES]
assert len(names) == len(set(names)), "duplicate case name"
manifest = {
    "manifest_version": 1,
    "generator": "tools/evidence-store/cli/capture-parity.ts",
    "sources": [
        "scripts/ci/schema.py",
        "scripts/ci/observation_notes.py",
        "scripts/ci/hosted_observations.py",
        "scripts/ci/test_hosted_observations.py",
    ],
    "loader_decode": "bytes.decode('utf-8') (strict, no BOM stripping), then schema.loads_strict",
    "cases": CASES,
}
emit("manifest.json", json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=True) + "\n")
print(f"{len(CASES)} cases")
`;

const FIXTURE_DIRECTORY = ["tests", "fixtures", "parity"] as const;

function listFiles(root: string): string[] {
  const files: string[] = [];
  const walk = (directory: string): void => {
    let entries;
    try {
      entries = readdirSync(directory, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.isFile()) files.push(relative(root, path).split(sep).join("/"));
    }
  };
  walk(root);
  return files.sort();
}

function capture(repositoryRoot: string, output: string, scratch: string): void {
  const home = join(scratch, "home-root");
  mkdirSync(home, { recursive: true });
  const environment: NodeJS.ProcessEnv = {
    PATH: process.env["PATH"] ?? "",
    HOME: home,
    USERPROFILE: home,
    LANG: "C.UTF-8",
    LC_ALL: "C.UTF-8",
    TZ: "UTC",
    GIT_CONFIG_NOSYSTEM: "1",
    // Every Git call fails fast: the hosted path's gate-input identity would
    // otherwise run `git ls-tree` against this repository.
    GIT_DIR: join(scratch, "no-repository"),
    GIT_CEILING_DIRECTORIES: scratch,
  };
  if (process.env["SYSTEMROOT"] !== undefined) environment["SYSTEMROOT"] = process.env["SYSTEMROOT"];
  const python = process.platform === "win32" ? "python" : "python3";
  const result = spawnSync(python, ["-I", "-", repositoryRoot, output, scratch], {
    cwd: repositoryRoot,
    env: environment,
    input: PYTHON_PROGRAM,
    encoding: "utf-8",
    shell: false,
    maxBuffer: 16 * 1024 * 1024,
  });
  if (result.error !== undefined) {
    throw new Error(`capture-parity: could not run ${python}: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`capture-parity: ${python} exited ${String(result.status)}\n${result.stderr}`);
  }
}

function main(argv: readonly string[]): number {
  const check = argv.includes("--check");
  const unknown = argv.filter((argument) => argument !== "--check");
  if (unknown.length > 0) {
    process.stderr.write(`capture-parity: unknown argument ${unknown.join(" ")}\nusage: capture-parity [--check]\n`);
    return 2;
  }
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const repositoryRoot = dirname(dirname(packageRoot));
  const fixtures = join(packageRoot, ...FIXTURE_DIRECTORY);
  const scratch = mkdtempSync(join(tmpdir(), "evidence-parity-"));
  try {
    const output = join(scratch, "out");
    mkdirSync(output);
    capture(repositoryRoot, output, scratch);
    const fresh = listFiles(output);
    const frozen = listFiles(fixtures);
    const differing = new Set<string>();
    for (const name of new Set([...fresh, ...frozen])) {
      const left = fresh.includes(name) ? readFileSync(join(output, name)) : undefined;
      const right = frozen.includes(name) ? readFileSync(join(fixtures, name)) : undefined;
      if (left === undefined || right === undefined || !left.equals(right)) differing.add(name);
    }
    if (check) {
      if (differing.size === 0) {
        process.stdout.write(`capture-parity: ${fresh.length} files match the Python contract\n`);
        return 0;
      }
      process.stderr.write(
        `capture-parity: ${differing.size} parity files differ from the Python contract:\n` +
          [...differing].sort().map((name) => `  ${name}\n`).join("") +
          "regenerate with: pnpm --dir tools/evidence-store exec tsx cli/capture-parity.ts\n",
      );
      return 1;
    }
    for (const name of frozen) {
      if (!fresh.includes(name)) rmSync(join(fixtures, name));
    }
    for (const name of fresh) {
      if (!differing.has(name)) continue;
      const target = join(fixtures, name);
      mkdirSync(dirname(target), { recursive: true });
      writeFileSync(target, readFileSync(join(output, name)));
    }
    removeEmptyDirectories(fixtures);
    process.stdout.write(`capture-parity: wrote ${fresh.length} files (${differing.size} changed)\n`);
    return 0;
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

function removeEmptyDirectories(root: string): void {
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const path = join(root, entry.name);
    removeEmptyDirectories(path);
    if (readdirSync(path).length === 0) rmSync(path, { recursive: true });
  }
}

try {
  process.exitCode = main(process.argv.slice(2));
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}
