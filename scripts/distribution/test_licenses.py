#!/usr/bin/env python3
"""Contract tests for `about.toml`, `licenses.sh`, and `native_notices.py`.

Run: `python scripts/distribution/test_licenses.py`. No network and no real
cargo or cargo-about: `licenses.sh` runs against fakes on `$CARGO` and
`$CARGO_ABOUT` and a throwaway checkout and registry.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

import copy  # noqa: E402
import hashlib  # noqa: E402
import importlib.util  # noqa: E402
import json  # noqa: E402
import re  # noqa: E402
import shutil  # noqa: E402
import subprocess  # noqa: E402
import tempfile  # noqa: E402
import textwrap  # noqa: E402
import tomllib  # noqa: E402
import unittest  # noqa: E402
from unittest import mock  # noqa: E402
from pathlib import Path  # noqa: E402

REPO = Path(HERE).parents[1]
SCRIPT = Path(HERE) / "licenses.sh"
FIXTURES = Path(HERE) / "fixtures" / "notices"


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


native_notices = load("native_notices", Path(HERE) / "native_notices.py")
import distribution  # noqa: E402  (on the path native_notices.py set up)

ACCEPTED = ["MIT", "Apache-2.0", "BSD-3-Clause", "ISC"]
PCRE2_HEADER = textwrap.dedent(
    """\
    /* PCRE is a library of functions to support regular expressions whose syntax
    Redistribution and use in source and binary forms, with or without
    POSSIBILITY OF SUCH DAMAGE.
    */
    int pcre2_compile(void);
    """
)
SLJIT_HEADER = textwrap.dedent(
    """\
    /*
     *    Stack-less Just-In-Time compiler
     * Redistribution and use in source and binary forms
     * ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
     */
    """
)


def package(root: Path, name: str, version: str, *, license="MIT", links=None, build_deps=(), files=None, local=False):
    """A `cargo metadata` package whose crate directory exists under `root`."""
    crate_dir = root / f"{name}-{version}"
    crate_dir.mkdir(parents=True, exist_ok=True)
    for relative, text in (files or {}).items():
        (crate_dir / relative).parent.mkdir(parents=True, exist_ok=True)
        (crate_dir / relative).write_text(text)
    source = None if local else "registry+https://github.com/rust-lang/crates.io-index"
    return {
        "id": f"{name} {version} ({source or 'path'})",
        "name": name,
        "version": version,
        "source": source,
        "license": license,
        "links": links,
        "manifest_path": str(crate_dir / "Cargo.toml"),
        "dependencies": [{"name": dep, "kind": "build"} for dep in build_deps],
        "targets": [{"kind": ["bin"] if local else ["lib"]}],
    }


def metadata(*packages, members=()):
    return {"packages": list(packages), "workspace_members": [entry["id"] for entry in members]}


def index(*crates):
    lines = ["## Rust crates", "", "### MIT License (`MIT`) (standard text)", "", "## Crate index", ""]
    lines += [f"- `{name} {version}`: {license}" for name, version, license in crates]
    return "\n".join(lines) + "\n"


def tree(*crates, root="demo-cli v0.1.0 (/work/demo/cli)"):
    return "\n".join([root, *(f"{name} v{version}" for name, version in crates)]) + "\n"


class AboutConfigTests(unittest.TestCase):
    def setUp(self):
        with (REPO / "about.toml").open("rb") as handle:
            self.config = tomllib.load(handle)

    def test_targets_equal_distribution_targets(self):
        self.assertEqual(self.config["targets"], list(distribution.TARGETS))

    def test_dev_and_build_dependencies_are_ignored(self):
        self.assertIs(self.config["ignore-dev-dependencies"], True)
        self.assertIs(self.config["ignore-build-dependencies"], True)

    def test_accepted_licenses_are_explicit_spdx_ids(self):
        accepted = self.config["accepted"]
        self.assertTrue(accepted)
        self.assertEqual(len(accepted), len(set(accepted)))
        for license_id in accepted:
            self.assertRegex(license_id, r"^[A-Za-z0-9.+-]+$", "no wildcard, expression, or NOASSERTION")
            self.assertNotIn("NOASSERTION", license_id)

    def test_clarifications_are_pinned_to_checksums(self):
        clarified = {key: value["clarify"] for key, value in self.config.items() if isinstance(value, dict) and "clarify" in value}
        self.assertIn("content-policy", clarified)
        for crate, clarify in clarified.items():
            self.assertTrue(clarify.get("files") or clarify.get("git"), crate)
            for entry in clarify.get("files", []):
                self.assertRegex(entry["checksum"], r"^[0-9a-f]{64}$", crate)

    def test_content_policy_clarification_matches_root_license(self):
        entry = self.config["content-policy"]["clarify"]["files"][0]
        path = (REPO / "content-policy" / "lib" / entry["path"]).resolve()
        self.assertEqual(path, REPO / "LICENSE")
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), entry["checksum"])

    def test_native_tables_do_not_overlap(self):
        catalogued = {library.crate for library in native_notices.NATIVE_LIBRARIES}
        self.assertFalse(catalogued & set(native_notices.REVIEWED_WITHOUT_NOTICE))


class NativeNoticeTests(unittest.TestCase):
    def test_a_cargo_command_that_hangs_is_bounded_and_reported(self):
        hang = self.tmp / "cargo"
        hang.write_text("#!/bin/sh\nexec sleep 30\n")
        hang.chmod(0o755)
        with mock.patch.dict(os.environ, {"CARGO": str(hang)}), \
                self.assertRaisesRegex(native_notices.NoticeError, r"cargo tree.*did not finish within 0\.5 seconds"):
            native_notices.run_cargo(self.tmp, "tree", timeout=0.5)

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.registry = self.tmp / "registry"

    def pcre2(self, **overrides):
        files = {
            "upstream/src/pcre2_compile.c": PCRE2_HEADER,
            "upstream/deps/sljit/sljit_src/sljitLir.c": SLJIT_HEADER,
            **overrides,
        }
        return package(self.registry, "pcre2-sys", "0.2.10", license="Unlicense OR MIT", build_deps=["cc"], files=files)

    def notices(self, packages, indexed, linked=None):
        crates = [(entry["name"], entry["version"]) for entry in packages]
        indexed_crates = [(entry["name"], entry["version"], entry["license"]) for entry in indexed]
        return native_notices.notices(index(*indexed_crates), [tree(*(linked or crates))], metadata(*packages), ACCEPTED)

    def test_pcre2_in_closure_appends_both_license_blocks(self):
        pcre2 = self.pcre2()
        text = self.notices([pcre2], [pcre2])
        self.assertIn("## Native libraries", text)
        self.assertIn("### PCRE2 (vendored by `pcre2-sys 0.2.10`)", text)
        self.assertIn("From `upstream/src/pcre2_compile.c`:", text)
        self.assertIn("POSSIBILITY OF SUCH DAMAGE.\n`````", text)
        self.assertNotIn("int pcre2_compile", text)
        self.assertIn("Stack-less Just-In-Time compiler", text)

    def test_closure_without_native_crates_appends_nothing(self):
        plain = package(self.registry, "itoa", "1.0.0")
        self.assertEqual(self.notices([plain], [plain]), "")

    def test_missing_license_source_fails(self):
        pcre2 = self.pcre2()
        (Path(pcre2["manifest_path"]).parent / "upstream/deps/sljit/sljit_src/sljitLir.c").unlink()
        with self.assertRaisesRegex(native_notices.NoticeError, "sljitLir.c cannot be read"):
            self.notices([pcre2], [pcre2])

    def test_license_block_missing_from_source_fails(self):
        pcre2 = self.pcre2(**{"upstream/src/pcre2_compile.c": "int main(void);\n"})
        with self.assertRaisesRegex(native_notices.NoticeError, "expected license block"):
            self.notices([pcre2], [pcre2])

    def test_unreviewed_native_crate_fails(self):
        unknown = package(self.registry, "zstd-sys", "2.0.0", links="zstd", build_deps=["cc"])
        with self.assertRaisesRegex(native_notices.NoticeError, "`zstd-sys 2.0.0` builds or links native code"):
            self.notices([unknown], [unknown])

    def test_reviewed_native_crate_needs_no_notice(self):
        alsa = package(self.registry, "alsa-sys", "0.4.0", links="alsa")
        grammar = package(self.registry, "tree-sitter-rust", "0.24.2", build_deps=["cc"])
        self.assertEqual(self.notices([alsa, grammar], [alsa, grammar]), "")

    def test_third_party_agpl_crate_fails(self):
        agpl = package(self.registry, "copyleft", "1.0.0", license="AGPL-3.0-only")
        with self.assertRaisesRegex(native_notices.NoticeError, "third-party AGPL-3.0-only"):
            self.notices([agpl], [agpl])

    def test_crate_omitted_by_cargo_about_is_added_with_its_license_file(self):
        listed = package(self.registry, "itoa", "1.0.0")
        omitted = package(self.registry, "dlib", "0.5.3", files={"LICENSE.txt": "Copyright (c) 2015 Victor Berger\n"})
        text = self.notices([listed, omitted], [listed])
        self.assertIn("## Additional Rust crates", text)
        self.assertIn("### dlib 0.5.3\n\nLicense: MIT (used under `MIT`).", text)
        self.assertIn("Copyright (c) 2015 Victor Berger", text)

    def test_omitted_crate_without_license_file_points_at_standard_text(self):
        listed = package(self.registry, "itoa", "1.0.0")
        omitted = package(self.registry, "libloading", "0.8.9", license="MIT OR Apache-2.0")
        text = self.notices([listed, omitted], [listed])
        self.assertIn("the standard text of `MIT` is above", text)

    def test_omitted_crate_without_any_license_text_fails(self):
        listed = package(self.registry, "itoa", "1.0.0")
        omitted = package(self.registry, "libloading", "0.8.9", license="ISC")
        with self.assertRaisesRegex(native_notices.NoticeError, r"no standard text for \['ISC'\]"):
            self.notices([listed, omitted], [listed])

    def test_omitted_crate_with_unaccepted_license_fails(self):
        listed = package(self.registry, "itoa", "1.0.0")
        omitted = package(self.registry, "gpl", "1.0.0", license="GPL-3.0-only")
        with self.assertRaisesRegex(native_notices.NoticeError, "no accepted alternative"):
            self.notices([listed, omitted], [listed])

    def test_read_tree_keeps_registry_and_git_crates_only(self):
        output = "demo-cli v0.1.0 (/work/cli)\nlib v0.1.0 (C:\\work\\lib)\nserde v1.0.0\nserde_derive v1.0.0 (proc-macro)\nforked v0.2.0 (https://github.com/o/r#abc) (*)\n"
        self.assertEqual(native_notices.read_tree(output), {("serde", "1.0.0"), ("serde_derive", "1.0.0"), ("forked", "0.2.0")})

    def test_choose_licenses(self):
        cases = {
            "MIT OR Apache-2.0": ["MIT"],
            "Apache-2.0/MIT": ["MIT"],
            "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT": ["MIT"],
            "MIT AND BSD-3-Clause": ["MIT", "BSD-3-Clause"],
            "(Apache-2.0 OR MIT) AND BSD-3-Clause": ["Apache-2.0", "MIT", "BSD-3-Clause"],
        }
        for expression, expected in cases.items():
            self.assertEqual(native_notices.choose_licenses(expression, ACCEPTED), expected, expression)

    def test_nullable_string_validator(self):
        """`_nullable_string`, once, on `license`: null is a value (no
        declared license); absence, other types, and empty are located."""
        plain = package(self.registry, "itoa", "1.0.0")
        self.assertEqual(native_notices.read_metadata(metadata({**plain, "license": None})), metadata({**plain, "license": None}))
        cases = {
            "absent": ({k: v for k, v in plain.items() if k != "license"}, r"packages\[0\].license: is required"),
            "wrong type": ({**plain, "license": ["MIT"]}, r"packages\[0\].license: must be a string, found list"),
            "empty": ({**plain, "license": ""}, r"packages\[0\].license: must not be empty"),
        }
        for name, (entry, pattern) in cases.items():
            with self.subTest(name), self.assertRaisesRegex(distribution.DistributionError, rf"^cargo metadata: {pattern}"):
                native_notices.read_metadata(metadata(entry))

    def test_each_metadata_field_rejects_an_invalid_shape(self):
        """One invalid shape per field this module reads, through the public
        `notices` entry point; the control is the fixture package itself."""
        plain = package(self.registry, "itoa", "1.0.0")
        self.assertEqual(self.notices([plain], [plain]), "")
        cases = {
            "workspace_members": (lambda m: m.update(workspace_members="x"), "workspace_members: must be a list"),
            "packages": (lambda m: m.update(packages=[]), "packages: must not be empty"),
            "id": (lambda m: m["packages"][0].update(id=None), r"\[0\].id: must not be null"),
            "name": (lambda m: m["packages"][0].pop("name"), r"\[0\].name: is required"),
            "version": (lambda m: m["packages"][0].update(version=1), r"\[0\].version: must be a string"),
            "manifest_path": (lambda m: m["packages"][0].update(manifest_path=""), r"\[0\].manifest_path: must not be empty"),
            "source": (lambda m: m["packages"][0].pop("source"), r"\[0\].source: is required"),
            "links": (lambda m: m["packages"][0].update(links=False), r"\[0\].links: must be a string, found boolean"),
            "dependencies": (lambda m: m["packages"][0].update(dependencies=[{"name": "cc"}]), r"dependencies\[0\].kind: is required"),
            "dependency name": (lambda m: m["packages"][0].update(dependencies=[{"name": 1, "kind": None}]), r"dependencies\[0\].name: must be a string"),
            "targets": (lambda m: m["packages"][0].update(targets=[]), r"\[0\].targets: must not be empty"),
            "target kind": (lambda m: m["packages"][0].update(targets=[{"kind": "bin"}]), r"targets\[0\].kind: must be a list"),
        }
        for name, (mutate, pattern) in cases.items():
            with self.subTest(name):
                broken = copy.deepcopy(metadata(plain))
                mutate(broken)
                with self.assertRaisesRegex(distribution.DistributionError, pattern):
                    native_notices.notices(index(("itoa", "1.0.0", "MIT")), [tree(("itoa", "1.0.0"))], broken, ACCEPTED)

    def test_about_config_fields(self):
        """The real `about.toml` is the control. A bare string once passed the
        list check (iterating it yields strings)."""
        self.assertEqual(native_notices.about_config(REPO), (list(distribution.TARGETS), tomllib.loads((REPO / "about.toml").read_text())["accepted"]))
        real = (REPO / "about.toml").read_text()
        cases = {
            "targets as a string": (re.sub(r"(?ms)^targets = \[.*?\]", 'targets = "aarch64-apple-darwin"', real), r"about.toml: targets: must be a list, found string"),
            "accepted with a non-string": (real.replace('    "MIT",\n', '    "MIT",\n    7,\n', 1), r"about.toml: accepted\[1\]: must be a string, found integer"),
            "duplicate key": ('targets = []\n' + real, r"about.toml: invalid TOML"),
        }
        for name, (text, pattern) in cases.items():
            with self.subTest(name):
                (self.tmp / "about.toml").write_text(text)
                with self.assertRaisesRegex(distribution.DistributionError, pattern):
                    native_notices.about_config(self.tmp)

    def test_manifest_path_requires_a_workspace_binary(self):
        cli = package(self.tmp, "demo-cli", "0.1.0", local=True)
        lib = {**package(self.tmp, "demo", "0.1.0", local=True), "targets": [{"kind": ["lib"]}]}
        found = metadata(cli, lib, members=[cli, lib])
        self.assertEqual(native_notices.manifest_path(found, "demo-cli"), cli["manifest_path"])
        with self.assertRaisesRegex(native_notices.NoticeError, "no binary target"):
            native_notices.manifest_path(found, "demo")
        with self.assertRaisesRegex(native_notices.NoticeError, "unknown package `nope`"):
            native_notices.manifest_path(found, "nope")


FAKE_CARGO = """\
import json, os, sys
fake = os.environ["FAKE_DIR"]
with open(os.path.join(fake, "cargo.log"), "a") as log:
    log.write(" ".join(sys.argv[1:]) + "\\n")
args = sys.argv[1:]
if args[0] == "metadata":
    name = "nodeps.json" if "--no-deps" in args else "full.json"
else:
    name = "tree-" + args[args.index("--target") + 1] + ".txt"
sys.stdout.write(open(os.path.join(fake, name)).read())
"""

FAKE_ABOUT = """\
import os, shutil, sys
fake = os.environ["FAKE_DIR"]
if sys.argv[1:] == ["--version"]:
    print("cargo-about " + os.environ["FAKE_ABOUT_VERSION"])
    sys.exit(0)
with open(os.path.join(fake, "about.log"), "a") as log:
    log.write(" ".join(sys.argv[1:]) + "\\n")
args = sys.argv[1:]
shutil.copyfile(os.path.join(fake, "crates.md"), args[args.index("-o") + 1])
if os.environ.get("FAKE_ABOUT_TOUCH_LOCK"):
    with open("Cargo.lock", "a") as lock:
        lock.write("# changed\\n")
"""


class LicensesScriptTests(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        self.fake = self.tmp / "fake"
        self.fake.mkdir()
        self.checkout = self.tmp / "checkout"
        (self.checkout / "cli").mkdir(parents=True)
        (self.checkout / "Cargo.lock").write_text("version = 4\n")
        shutil.copyfile(REPO / "about.toml", self.checkout / "about.toml")
        self.output = self.tmp / "out" / "THIRD-PARTY-NOTICES.md"

        self.cli = package(self.checkout, "demo-cli", "0.1.0", local=True)
        self.cli["manifest_path"] = str(self.checkout / "cli" / "Cargo.toml")
        registry = self.tmp / "registry"
        files = {"upstream/src/pcre2_compile.c": PCRE2_HEADER, "upstream/deps/sljit/sljit_src/sljitLir.c": SLJIT_HEADER}
        self.pcre2 = package(registry, "pcre2-sys", "0.2.10", license="Unlicense OR MIT", build_deps=["cc"], files=files)
        self.itoa = package(registry, "itoa", "1.0.0")
        self.write_fakes(self.itoa, self.pcre2)

    def write_fakes(self, *crates):
        (self.fake / "nodeps.json").write_text(json.dumps(metadata(self.cli, members=[self.cli])))
        (self.fake / "full.json").write_text(json.dumps(metadata(self.cli, *crates, members=[self.cli])))
        for target in distribution.TARGETS:
            (self.fake / f"tree-{target}.txt").write_text(tree(*[(c["name"], c["version"]) for c in crates]))
        (self.fake / "crates.md").write_text(index(*[(c["name"], c["version"], c["license"]) for c in crates]))
        for name, body in (("cargo", FAKE_CARGO), ("cargo-about", FAKE_ABOUT)):
            path = self.fake / name
            path.write_text(f"#!{sys.executable} -I\n{body}")
            path.chmod(0o755)

    def run_script(self, *args, version="0.8.4", **env):
        environment = {
            **os.environ,
            "CARGO": str(self.fake / "cargo"),
            "CARGO_ABOUT": str(self.fake / "cargo-about"),
            "PYTHON": sys.executable,
            "FAKE_DIR": str(self.fake),
            "FAKE_ABOUT_VERSION": version,
            **env,
        }
        if not args:
            args = (str(self.checkout), "demo-cli", str(self.output))
        return subprocess.run(["bash", str(SCRIPT), *args], env=environment, capture_output=True, text=True)

    def calls(self, name):
        log = self.fake / f"{name}.log"
        return log.read_text().splitlines() if log.exists() else []

    def test_refuses_another_cargo_about_version_before_any_work(self):
        result = self.run_script(version="0.8.3")
        self.assertEqual(result.returncode, 1)
        self.assertIn("need cargo-about 0.8.4, found 'cargo-about 0.8.3'", result.stderr)
        self.assertEqual(self.calls("cargo") + self.calls("about"), [])
        self.assertFalse(self.output.exists())

    def test_refuses_missing_cargo_about(self):
        result = self.run_script(CARGO_ABOUT=str(self.tmp / "absent"))
        self.assertEqual(result.returncode, 1)
        self.assertIn("not found", result.stderr)

    def test_refuses_unknown_package_before_generating(self):
        result = self.run_script(str(self.checkout), "nope-cli", str(self.output))
        self.assertEqual(result.returncode, 1)
        self.assertIn("unknown package `nope-cli`", result.stderr)
        self.assertEqual(self.calls("about"), [])
        self.assertTrue(all("--no-deps" in call for call in self.calls("cargo")))

    def test_usage_error(self):
        result = self.run_script(str(self.checkout), "demo-cli")
        self.assertEqual(result.returncode, 2)
        self.assertIn("usage: licenses.sh", result.stderr)

    def test_writes_header_crates_and_native_notices(self):
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        text = self.output.read_text()
        header = (FIXTURES / "header.md").read_text()
        self.assertTrue(text.startswith(header), text[: len(header)])
        self.assertIn("- `pcre2-sys 0.2.10`: Unlicense OR MIT", text)
        self.assertIn("### PCRE2 (vendored by `pcre2-sys 0.2.10`)", text)
        self.assertNotIn("## Additional Rust crates", text)
        (generate,) = self.calls("about")
        self.assertRegex(generate, r"^generate --locked --fail -c \S+/about.toml -m \S+/cli/Cargo.toml -o \S+ \S+/about.hbs$")
        for target in distribution.TARGETS:
            self.assertTrue(any(f"tree --locked -e normal -p demo-cli --target {target}" in call for call in self.calls("cargo")))

    def test_without_native_crates_appends_nothing(self):
        self.write_fakes(self.itoa)
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        text = self.output.read_text()
        self.assertNotIn("## Native libraries", text)
        self.assertTrue(text.endswith("- `itoa 1.0.0`: MIT\n"), text[-200:])

    def test_fails_when_lockfile_changes(self):
        result = self.run_script(FAKE_ABOUT_TOUCH_LOCK="1")
        self.assertEqual(result.returncode, 1)
        self.assertIn("Cargo.lock changed", result.stderr)
        self.assertFalse(self.output.exists())

    def test_fails_on_empty_cargo_about_output(self):
        (self.fake / "crates.md").write_text("")
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertFalse(self.output.exists())

    def run_notices(self, *args):
        """`native_notices.py` itself, with the fake cargo."""
        environment = {**os.environ, "CARGO": str(self.fake / "cargo"), "FAKE_DIR": str(self.fake)}
        return subprocess.run([sys.executable, str(Path(HERE) / "native_notices.py"), *args], env=environment,
                              capture_output=True, text=True)

    def test_cli_manifest_control_and_malformed_metadata(self):
        result = self.run_notices("manifest", str(self.checkout), "demo-cli")
        self.assertEqual((result.returncode, result.stdout.strip()), (0, self.cli["manifest_path"]), result.stderr)
        (self.fake / "nodeps.json").write_text((self.fake / "nodeps.json").read_text() + "\n[]")
        result = self.run_notices("manifest", str(self.checkout), "demo-cli")
        self.assertEqual((result.returncode, result.stdout), (1, ""))
        self.assertIn(f"cargo metadata ({self.checkout}): invalid JSON", result.stderr)

    def test_cli_malformed_about_config_runs_no_cargo(self):
        (self.checkout / "about.toml").write_text("accepted = []\n" + (REPO / "about.toml").read_text())
        result = self.run_notices("render", str(self.checkout), "demo-cli", str(self.fake / "crates.md"))
        self.assertEqual((result.returncode, result.stdout), (1, ""))
        self.assertIn(f"{self.checkout / 'about.toml'}: invalid TOML", result.stderr)
        self.assertEqual(self.calls("cargo"), [])

    def test_fails_when_native_license_is_missing(self):
        (Path(self.pcre2["manifest_path"]).parent / "upstream/src/pcre2_compile.c").unlink()
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertIn("pcre2_compile.c cannot be read", result.stderr)
        self.assertFalse(self.output.exists())


class TemplateContractTests(unittest.TestCase):
    """The template lines `native_notices.py` parses."""

    def test_template_renders_the_parsed_formats(self):
        template = (Path(HERE) / "about.hbs").read_text()
        self.assertIn("\n" + native_notices.INDEX_HEADING + "\n", template)
        self.assertIn("- `{{package.name}} {{package.version}}`: {{{license}}}", template)
        self.assertIn("### {{name}} (`{{id}}`){{#unless source_path}} (standard text){{/unless}}", template)
        self.assertTrue(re.search(r'\(eq id "AGPL-3\.0-only"\)', template))


if __name__ == "__main__":
    unittest.main()
