"""Deterministic evidence tests; no registry or provider access."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location("collector", Path(__file__).with_name("collect.py"))
collector = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(collector)


def metadata(root):
    deps = [{"name": "shared", "source": "registry+example", "req": req,
             "kind": None, "rename": "alias", "optional": False,
             "uses_default_features": False, "features": ["derive"], "target": None}
            for req in ("^1", ">=1, <2")]
    packages = [dict(id=name, name=name, version="0.1.0", source=None,
                     manifest_path=str(root / name / "Cargo.toml"), dependencies=[dep])
                for name, dep in zip(("a", "b"), deps)]
    packages += [dict(id=identity, name="shared", version="1.2.0", source=source, dependencies=[])
                 for identity, source in (("shared-registry", "registry+example"), ("shared-git", "git+example"))]
    return dict(packages=packages, workspace_members=["a", "b"], resolve={"nodes": [
        dict(id="a", features=[], deps=[dict(name="alias", pkg="shared-registry", dep_kinds=[dict(kind=None, target=None)])]),
        dict(id="shared-git", features=[], deps=[dict(name="shared", pkg="shared-registry", dep_kinds=[dict(kind="build", target="cfg(unix)")])])
    ]})


class EvidenceTests(unittest.TestCase):
    def test_single_version_alignment_and_source_divergence(self):
        reports = collector.summarize(metadata(Path("/fixture")))
        group = reports["workspace-requirements-by-dependency"][0]
        self.assertEqual(group["package_count"], 2)
        self.assertEqual(group["requirements"], [">=1, <2", "^1"])
        self.assertFalse(group["declarations"][0]["uses_default_features"])
        duplicates = reports["resolved-duplicate-identities"]
        self.assertEqual(duplicates[0]["version_count"], 1)
        self.assertEqual(duplicates[0]["source_count"], 2)
        parents = reports["reverse-dependency-edges"]["shared-registry"]
        self.assertEqual({p["from_id"] for p in parents}, {"a", "shared-git"})
        self.assertEqual(parents[0]["name"], "alias")
        self.assertEqual(parents[1]["dep_kinds"][0]["kind"], "build")

    def test_invalid_json_and_failed_commands_never_publish_artifacts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, code in (("bad-json", "print('not json')"),
                               ("failed", "print('{}'); raise SystemExit(2)"),
                               ("metadata-default", "print('{}')")):
                record, value = collector.run_command(root, root, name, [sys.executable, "-c", code], json_output=True)
                self.assertEqual(record["status"], "failed")
                self.assertIsNone(value)
                self.assertNotIn("artifact", record)
                self.assertTrue((root / record["stdout"]).exists())

    def test_nonmember_path_dependencies_retain_distinct_sources(self):
        data = metadata(Path("/fixture"))
        for package in data["packages"][2:]:
            package["source"] = None
            package["manifest_path"] = f"/external/{package['id']}/Cargo.toml"
        group = collector.summarize(data)["resolved-duplicate-identities"][0]
        self.assertEqual(group["source_count"], 2)
        self.assertEqual(group["version_count"], 1)

    def test_timeout_and_missing_executable_are_diagnostics(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            record, _ = collector.run_command(root, root, "missing", [str(root / "absent")])
            self.assertEqual(record["status"], "failed")
            with patch.object(collector.subprocess, "run", side_effect=subprocess.TimeoutExpired("cargo", 1)):
                record, _ = collector.run_command(root, root, "timeout", ["cargo"], timeout=1)
            self.assertIn("timed out", record["error"])

    def test_partial_runs_preserve_inheritance_and_cannot_reuse_artifacts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[workspace]\nmembers = ["a", "b"]\n[workspace.dependencies]\nshared = "1"\n')
            (root / "Cargo.lock").write_text("fixture lockfile")
            for member in ("a", "b"):
                (root / member).mkdir()
                (root / member / "Cargo.toml").write_text('[dependencies]\nalias = { workspace = true, features = ["derive"] }\n')

            def fake_run(repo, output, name, argv, **kwargs):
                failed = name in {"metadata-default", "metadata-all-features", "outdated"}
                return (dict(status="failed" if failed else "ok", command=argv, exit_status=1 if failed else 0),
                        metadata(root) if name == "metadata-declarations" else None)

            with patch.object(collector, "run_command", side_effect=fake_run):
                first, index = collector.collect(root)
                (first / "metadata-default.json").write_text("stale")
                second, index = collector.collect(root)
            self.assertNotEqual(first, second)
            self.assertFalse((second / "metadata-default.json").exists())
            self.assertEqual(index["status"], "partial")
            self.assertTrue(index["lockfile_unchanged"])
            self.assertEqual(index["derived"]["metadata-default"]["status"], "skipped")
            manifests = json.loads((second / "manifests.json").read_text())
            inherited = [m for m in manifests if "dependencies" in m["manifest"]]
            self.assertTrue(inherited[0]["manifest"]["dependencies"]["alias"]["workspace"])
            self.assertTrue((second / "metadata-declarations-workspace-requirements-by-dependency.json").exists())
            self.assertFalse((second / "metadata-declarations-resolved-dependency-edges.json").exists())


if __name__ == "__main__":
    unittest.main()
