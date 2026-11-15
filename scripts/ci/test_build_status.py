#!/usr/bin/env python3
"""Contract tests for the consumer's pre-download build-status check.

Every case mutates one field of `CONTROL`, a status in the exact shape the build
owner publishes: `ci-build-archive.rs::BuildStatus` serialized in its field
order, plus the `stage_seconds` that `publish.cjs` merges in. The control
staying `ok` is what makes each refusal mean something.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import build_status  # noqa: E402
from build_status import Cell, StatusError, Verdict  # noqa: E402

CELL = Cell(
    package="sniff-detection-cli",
    key="abc0123456789def",
    producer="ubuntu-latest",
    artifact="build-sniff-detection-cli-ubuntu-latest-abc0123456789def",
)

CONTROL = (
    '{"schema_version":1,"key":"abc0123456789def","package":"sniff-detection-cli",'
    '"producer":"ubuntu-latest",'
    '"artifact":"build-sniff-detection-cli-ubuntu-latest-abc0123456789def",'
    '"result":"success","stage":"produce",'
    '"consumers":[{"environment":"ubuntu-latest","gate":"L1"},'
    '{"environment":"wsl2-ubuntu","gate":"L1"}],'
    '"digest":"0f1e2d3c4b5a69788796a5b4c3d2e1f0",'
    '"timings":{"setup_ms":12,"compile_archive_ms":91000,"sidecars_ms":0,'
    '"inventory_ms":420,"checksum_ms":88,"total_ms":91520},'
    '"stage_seconds":{"upload_seconds":3,"queue_seconds":41}}'
)

FILE = Path("artifacts") / "build-status.json"
MISSING = object()


def mutated(field: str, value: Any) -> str:
    """`CONTROL` with one field replaced, or removed when `value` is `MISSING`."""
    document = json.loads(CONTROL)
    if value is MISSING:
        del document[field]
    else:
        document[field] = value
    return json.dumps(document)


class Case(unittest.TestCase):
    def judge(self, text: str) -> Verdict:
        return build_status.parse(text, FILE, CELL)

    def refuses(self, text: str, field: str) -> StatusError:
        with self.assertRaises(StatusError) as caught:
            self.judge(text)
        self.assertEqual(FILE, caught.exception.file)
        self.assertEqual(field, caught.exception.field)
        self.assertIn(str(FILE), str(caught.exception))
        self.assertIn(field, str(caught.exception))
        return caught.exception


class ControlTests(Case):
    def test_the_published_shape_lets_the_download_proceed(self) -> None:
        verdict = self.judge(CONTROL)
        self.assertTrue(verdict.ok)
        self.assertIsNone(verdict.detail)

    def test_the_expected_status_artifact_name_is_exact(self) -> None:
        self.assertEqual(
            "build-status-sniff-detection-cli-ubuntu-latest-abc0123456789def",
            CELL.status_artifact,
        )

    def test_uninterpreted_fields_decide_nothing(self) -> None:
        # Authoritative archive verification checks these after the download.
        for field in ("digest", "timings", "consumers", "stage_seconds"):
            with self.subTest(field=field):
                self.assertTrue(self.judge(mutated(field, MISSING)).ok)
                self.assertTrue(self.judge(mutated(field, 5)).ok)


class RequiredStringTests(Case):
    """The shared validator, walked once through every invalid shape."""

    def test_missing_null_wrong_type_and_empty_are_refused(self) -> None:
        shapes = {
            "missing": MISSING,
            "null": None,
            "wrong type": 7,
            "empty": "",
        }
        for label, value in shapes.items():
            with self.subTest(shape=label):
                error = self.refuses(mutated("package", value), "package")
                self.assertIn(
                    {"missing": "missing", "null": "null", "wrong type": "int", "empty": "empty"}[label],
                    error.problem,
                )

    def test_every_wired_field_uses_the_validator(self) -> None:
        for field in build_status.REQUIRED_STRINGS:
            with self.subTest(field=field):
                self.assertTrue(self.judge(mutated(field, json.loads(CONTROL)[field])).ok)
                self.refuses(mutated(field, ["x"]), field)


class SchemaVersionTests(Case):
    def test_only_the_integer_one_is_accepted(self) -> None:
        self.assertTrue(self.judge(mutated("schema_version", 1)).ok)
        for value in ("1", True, 2, 1.0, MISSING, None):
            with self.subTest(value=value):
                self.refuses(mutated("schema_version", value), "schema_version")


class ResultTests(Case):
    def test_failure_and_cancelled_refuse_as_producer_failed(self) -> None:
        for result, stage in (("failure", "compile"), ("cancelled", "upload")):
            with self.subTest(result=result):
                document = json.loads(mutated("result", result))
                document["stage"] = stage
                verdict = self.judge(json.dumps(document))
                self.assertFalse(verdict.ok)
                self.assertTrue(verdict.message.startswith("not run: producer failed"))
                for named in (CELL.package, CELL.key, CELL.producer, stage, result):
                    self.assertIn(named, verdict.message)

    def test_an_unknown_result_fails_closed(self) -> None:
        for value in ("skipped", "Success"):
            with self.subTest(value=value):
                self.refuses(mutated("result", value), "result")

    def test_stage_never_authorizes_success(self) -> None:
        document = json.loads(mutated("result", "failure"))
        document["stage"] = "success"
        self.assertFalse(self.judge(json.dumps(document)).ok)


class IdentityTests(Case):
    def test_each_identity_field_must_name_this_cell(self) -> None:
        for field in build_status.IDENTITY_FIELDS:
            with self.subTest(field=field):
                error = self.refuses(mutated(field, "someone-else"), field)
                self.assertIn("someone-else", error.problem)
                self.assertIn(getattr(CELL, field), error.problem)


class DetailTests(Case):
    def failed(self, detail: Any) -> str:
        document = json.loads(mutated("result", "failure"))
        if detail is not MISSING:
            document["detail"] = detail
        return json.dumps(document)

    def test_absent_and_null_mean_no_detail(self) -> None:
        for value in (MISSING, None):
            with self.subTest(value=value):
                verdict = self.judge(self.failed(value))
                self.assertIsNone(verdict.detail)
                self.assertNotIn("detail:", verdict.message)

    def test_any_string_is_preserved_including_empty(self) -> None:
        for value in ("", "x"):
            with self.subTest(value=value):
                verdict = self.judge(self.failed(value))
                self.assertEqual(value, verdict.detail)
                self.assertIn(f"detail: {json.dumps(value)}", verdict.message)

    def test_a_non_string_detail_is_refused(self) -> None:
        self.refuses(self.failed(5), "detail")


class LoaderTests(Case):
    def test_strict_loader_shapes_are_refused_at_the_document(self) -> None:
        shapes = {
            "duplicate key": CONTROL[:-1] + ',"result":"success"}',
            "trailing garbage": CONTROL + " trailing",
            "invalid JSON": CONTROL[:-1],
            "NaN": mutated("stage_seconds", MISSING)[:-1] + ',"stage_seconds":NaN}',
            "Infinity": mutated("stage_seconds", MISSING)[:-1] + ',"stage_seconds":Infinity}',
            "second document": CONTROL + "\n" + CONTROL,
        }
        for label, text in shapes.items():
            with self.subTest(shape=label):
                self.refuses(text, "(document)")

    def test_a_non_object_document_is_refused(self) -> None:
        self.refuses("[]", "(document)")


class FileFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="build-status-"))
        self.addCleanup(shutil.rmtree, self.root, True)
        self.status = self.root / CELL.status_artifact / "build-status.json"

    def write(self, text: str) -> Path:
        self.status.parent.mkdir(parents=True, exist_ok=True)
        self.status.write_text(text, encoding="utf-8")
        return self.status


class FileTests(FileFixture):
    def test_an_absent_status_is_an_unknown_outcome_naming_the_owner(self) -> None:
        verdict = build_status.check(self.status, CELL)
        self.assertFalse(verdict.ok)
        self.assertTrue(verdict.message.startswith("not run: producer outcome unknown"))
        self.assertNotIn("producer failed", verdict.message)
        self.assertIn("build (ubuntu-latest)", verdict.message)
        self.assertIn(CELL.status_artifact, verdict.message)
        self.assertIsNone(verdict.detail)

    def test_a_written_control_is_read_as_utf8(self) -> None:
        self.write(mutated("result", "failure")[:-1] + ',"detail":"caf\\u00e9 — ok"}')
        verdict = build_status.check(self.status, CELL)
        self.assertEqual("café — ok", verdict.detail)

    def test_bytes_that_are_not_utf8_are_located(self) -> None:
        self.status.parent.mkdir(parents=True)
        self.status.write_bytes(b'{"stage":"\xff"}')
        with self.assertRaises(StatusError) as caught:
            build_status.check(self.status, CELL)
        self.assertEqual(self.status, caught.exception.file)


class CliTests(FileFixture):
    """The CLI is spawned only to prove its wiring: exit code and location."""

    def run_cli(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(HERE / "build_status.py"),
                "--status", str(self.status),
                "--package", CELL.package,
                "--key", CELL.key,
                "--producer", CELL.producer,
                "--artifact", CELL.artifact,
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
        )

    def test_the_control_exits_zero(self) -> None:
        self.write(CONTROL)
        completed = self.run_cli()
        self.assertEqual(0, completed.returncode, completed.stderr)

    def test_malformed_json_exits_nonzero_naming_the_file(self) -> None:
        self.write(CONTROL[:-1])
        completed = self.run_cli()
        self.assertEqual(1, completed.returncode)
        self.assertIn(str(self.status), completed.stderr)
        self.assertIn("(document)", completed.stderr)

    def test_a_mismatched_key_exits_nonzero_naming_file_and_field(self) -> None:
        self.write(mutated("key", "ffff000011112222"))
        completed = self.run_cli()
        self.assertEqual(1, completed.returncode)
        self.assertIn(f"{self.status}: key:", completed.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
