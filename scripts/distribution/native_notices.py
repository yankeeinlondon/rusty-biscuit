#!/usr/bin/env python3
"""Notices cargo-about cannot produce for a distributed package.

`licenses.sh` renders the Rust crate notices with cargo-about and this module
adds what that output lacks, failing rather than shipping incomplete notices:

- Crates Cargo links that cargo-about's graph omits. cargo-about 0.8.4
  resolves features itself and drops some target-specific optional
  dependencies (for example fontique's Linux-only `yeslogic-fontconfig-sys`),
  so the closure is checked against `cargo tree -e normal` for every target in
  `about.toml`. Each omitted crate is listed with its packaged license files,
  or with a pointer to the standard text of its accepted license above.
- Native libraries. Some crates compile C sources they vendor into the binary,
  and those libraries' licenses are not part of the crate's metadata; they are
  copied from the vendored sources so they always match the locked version.

Commands:

- `manifest <checkout-root> <package>` prints the package's manifest path, or
  fails when the package is not a workspace member with a binary target.
- `render <checkout-root> <package> <crates-markdown>` prints the extra
  sections (nothing when none apply). It fails when a license source or block
  is missing, when an omitted crate's license is not accepted or has no text,
  when a crate builds or links native code and has not been reviewed below,
  or when a third-party crate is under AGPL-3.0-only (the license sections
  omit that license as the repository's own).

`$CARGO` overrides `cargo`.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
from distribution import DistributionError, Document, Fields, load_json, load_toml  # noqa: E402

FIRST_PARTY_LICENSE = "AGPL-3.0-only"
INDEX_HEADING = "## Crate index"
INDEX_LINE = re.compile(r"^- `(?P<name>[^ `]+) (?P<version>[^ `]+)`: (?P<license>.+)$")
STANDARD_HEADING = re.compile(r"^### .* \(`(?P<id>[^`]+)`\) \(standard text\)$")
# `cargo tree --prefix none --format {p}` lines: `name vX.Y.Z`, then an optional
# source such as `(/path)`, `(C:\path)`, or `(https://...)`, then markers like `(*)`.
TREE_LINE = re.compile(r"^(?P<name>\S+) v(?P<version>\S+)(?P<rest>.*)$")
LOCAL_SOURCE = re.compile(r"^\s*\((/|[A-Za-z]:[\\/])")
LICENSE_FILE = re.compile(r"^(licen[cs]e|copying|notice)", re.IGNORECASE)


class NoticeError(Exception):
    pass


@dataclass(frozen=True)
class Excerpt:
    """Lines of `path` (relative to the crate root) from the first line
    containing `start` through the next line containing `end`; the whole file
    when both are `None`."""

    path: str
    start: str | None = None
    end: str | None = None


@dataclass(frozen=True)
class NativeLibrary:
    crate: str
    library: str
    sources: tuple[Excerpt, ...]


@dataclass(frozen=True)
class Crate:
    name: str
    version: str
    license: str
    metadata: Mapping[str, Any]

    @property
    def label(self) -> str:
        return f"`{self.name} {self.version}`"


# pcre2-sys does not package the upstream PCRE2 LICENCE file; the same terms
# head every PCRE2 source file and every sljit file (the JIT compiler, which
# pcre2-sys enables on all distributed targets).
NATIVE_LIBRARIES: tuple[NativeLibrary, ...] = (
    NativeLibrary(
        "pcre2-sys",
        "PCRE2",
        (
            Excerpt("upstream/src/pcre2_compile.c", "PCRE is a library of functions", "POSSIBILITY OF SUCH DAMAGE."),
            Excerpt("upstream/deps/sljit/sljit_src/sljitLir.c", "Stack-less Just-In-Time compiler", "POSSIBILITY OF SUCH DAMAGE."),
        ),
    ),
    NativeLibrary("onig_sys", "Oniguruma", (Excerpt("oniguruma/COPYING"),)),
    NativeLibrary("tree-sitter", "ICU (Unicode headers in the tree-sitter runtime)", (Excerpt("src/unicode/LICENSE"),)),
    NativeLibrary("aws-lc-sys", "AWS-LC", (Excerpt("LICENSE"),)),
    NativeLibrary("ring", "BoringSSL-derived code in ring", (Excerpt("LICENSE-BoringSSL"),)),
)

# Crates that declare `links` or build with `cc`/`cmake` but ship no native code
# whose license is outside their own metadata. A crate missing from both this
# table and NATIVE_LIBRARIES fails `render` until someone reviews it.
REVIEWED_WITHOUT_NOTICE: Mapping[str, str] = {
    "alsa-sys": "links the system ALSA library at run time; nothing vendored",
    "coreaudio-sys": "bindings to macOS system frameworks",
    "yeslogic-fontconfig-sys": "loads or links the system fontconfig; nothing vendored",
    "libsqlite3-sys": "bundled SQLite is in the public domain",
    "blake3": "C and assembly from the BLAKE3 project, under the crate's license",
    "rav1e": "assembly is the crate's own, under its license",
    "av-scenechange": "assembly is the crate's own, under its license",
    "signal-hook": "C helper is the crate's own",
    "objc2-exception-helper": "Objective-C helper is the crate's own",
    "mac-notification-sys": "Objective-C helper is the crate's own",
    "aws-lc-rs": "`links` only; the native code is aws-lc-sys",
    "rayon-core": "`links` only; no native code",
    "tree-sitter-language": "`links` only; no native code",
    "wasm-bindgen-shared": "`links` only; no native code",
}
# Generated grammar parsers and scanners, under the grammar crate's license.
REVIEWED_PREFIXES = ("tree-sitter-",)
NATIVE_BUILD_TOOLS = frozenset({"cc", "cmake"})


# `cargo metadata` and `cargo tree` may fetch the registry on a cold runner.
CARGO_TIMEOUT_SECONDS = 1800


def run_cargo(root: Path, *args: str, timeout: float = CARGO_TIMEOUT_SECONDS) -> str:
    command = [os.environ.get("CARGO") or "cargo", *args]
    try:
        result = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        raise NoticeError(f"`{' '.join(command)}` did not finish within {timeout:g} seconds") from None
    if result.returncode != 0:
        raise NoticeError(f"`{' '.join(command)}` failed with exit code {result.returncode}")
    return result.stdout


def cargo_metadata(root: Path, *args: str) -> dict[str, Any]:
    source = f"cargo metadata ({root})"
    return read_metadata(load_json(run_cargo(root, "metadata", "--locked", "--format-version", "1", *args), source=source),
                         source=source)


def _nullable_string(fields: Fields, path: list[Any]) -> str | None:
    """Cargo writes null for an unset string field; absence is malformed."""
    parent = fields.mapping(path[:-1])
    if path[-1] not in parent:
        raise fields.error(path, "is required (cargo metadata writes null when unset)")
    return None if parent[path[-1]] is None else fields.string(path)


def read_metadata(value: Any, *, source: str = "cargo metadata") -> dict[str, Any]:
    """`value` after validating every `cargo metadata` field this module
    reads, so the functions below index it directly."""
    fields = Fields(Document(source, value))
    # Empty members cannot hide anything: `manifest_path` then reports the package unknown.
    fields.strings(["workspace_members"], allow_empty=True)
    for index in range(len(fields.records(["packages"], allow_empty=False))):
        base = ["packages", index]
        for key in ("id", "name", "version", "manifest_path"):
            fields.string([*base, key])
        for key in ("source", "license", "links"):
            _nullable_string(fields, [*base, key])
        for dependency in range(len(fields.records([*base, "dependencies"], allow_empty=True))):
            fields.string([*base, "dependencies", dependency, "name"])
            _nullable_string(fields, [*base, "dependencies", dependency, "kind"])
        for target in range(len(fields.records([*base, "targets"], allow_empty=False))):
            fields.strings([*base, "targets", target, "kind"], allow_empty=False)
    return value


def cargo_tree(root: Path, package: str, target: str) -> str:
    return run_cargo(
        root, "tree", "--locked", "-e", "normal", "-p", package, "--target", target, "--prefix", "none", "--format", "{p}"
    )


def about_config(root: Path) -> tuple[list[str], list[str]]:
    """The `targets` and `accepted` lists of `<root>/about.toml`."""
    path = root / "about.toml"
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise NoticeError(f"{path}: cannot read: {error}") from None
    fields = Fields(Document(str(path), load_toml(text, source=str(path))))
    return fields.strings(["targets"], allow_empty=False), fields.strings(["accepted"], allow_empty=False)


def manifest_path(metadata: Mapping[str, Any], package: str) -> str:
    read_metadata(metadata)
    members = set(metadata["workspace_members"])
    for candidate in metadata["packages"]:
        if candidate["id"] in members and candidate["name"] == package:
            if any("bin" in target["kind"] for target in candidate["targets"]):
                return candidate["manifest_path"]
            raise NoticeError(f"package `{package}` has no binary target")
    raise NoticeError(f"unknown package `{package}`: not a workspace member")


def read_index(markdown: str) -> dict[tuple[str, str], str]:
    """`(name, version)` to license expression, from the template's crate index."""
    lines = markdown.splitlines()
    if INDEX_HEADING not in lines:
        raise NoticeError(f"the cargo-about output has no `{INDEX_HEADING}` section")
    crates = {}
    for line in lines[lines.index(INDEX_HEADING) + 1 :]:
        if line.startswith("#"):
            break
        if not line.strip():
            continue
        match = INDEX_LINE.match(line)
        if match is None:
            raise NoticeError(f"unexpected crate index line: {line!r}")
        crates[(match["name"], match["version"])] = match["license"]
    if not crates:
        raise NoticeError("the crate index lists no third-party crates")
    return crates


def standard_texts(markdown: str) -> set[str]:
    return {match["id"] for line in markdown.splitlines() if (match := STANDARD_HEADING.match(line))}


def read_tree(output: str) -> set[tuple[str, str]]:
    """Third-party `(name, version)` pairs; path crates are this repository's own."""
    crates = set()
    for line in output.splitlines():
        if not line.strip():
            continue
        match = TREE_LINE.match(line.strip())
        if match is None:
            raise NoticeError(f"unexpected `cargo tree` line: {line!r}")
        if not LOCAL_SOURCE.match(match["rest"]):
            crates.add((match["name"], match["version"]))
    return crates


def third_party(metadata: Mapping[str, Any], keys: Iterable[tuple[str, str]]) -> dict[tuple[str, str], Mapping[str, Any]]:
    by_key: dict[tuple[str, str], list[Mapping[str, Any]]] = {}
    for entry in metadata["packages"]:
        if entry["source"] is not None:
            by_key.setdefault((entry["name"], entry["version"]), []).append(entry)
    found = {}
    for key in keys:
        matches = by_key.get(key, [])
        if len(matches) != 1:
            raise NoticeError(f"`{key[0]} {key[1]}` matches {len(matches)} third-party packages in `cargo metadata`")
        found[key] = matches[0]
    return found


def choose_licenses(expression: str, accepted: Sequence[str]) -> list[str]:
    """The license ids an omitted crate is shipped under: of the top-level `OR`
    alternatives, the first (in `accepted` order) whose `AND` terms are all
    accepted. A parenthesized expression must have every id accepted."""
    normalized = " ".join(expression.replace("/", " OR ").split())
    if "(" in normalized:
        ids = [token for token in re.split(r"[\s()]+", normalized) if token and token not in ("OR", "AND", "WITH")]
        rejected = [license_id for license_id in ids if license_id not in accepted]
        if rejected:
            raise NoticeError(f"license expression {expression!r} needs unaccepted {rejected}")
        return list(dict.fromkeys(ids))
    alternatives = [alternative.split(" AND ") for alternative in normalized.split(" OR ")]
    usable = [terms for terms in alternatives if all(term in accepted for term in terms)]
    if not usable:
        raise NoticeError(f"license expression {expression!r} has no accepted alternative")
    return min(usable, key=lambda terms: [accepted.index(term) for term in terms])


def check(crates: Sequence[Crate]) -> None:
    catalogued = {library.crate for library in NATIVE_LIBRARIES}
    problems = []
    for crate in crates:
        if FIRST_PARTY_LICENSE in crate.license:
            problems.append(f"{crate.label} is a third-party {FIRST_PARTY_LICENSE} crate, which the license sections omit")
        builds_native = crate.metadata["links"] is not None or any(
            dep["kind"] == "build" and dep["name"] in NATIVE_BUILD_TOOLS for dep in crate.metadata["dependencies"]
        )
        reviewed = crate.name in REVIEWED_WITHOUT_NOTICE or crate.name.startswith(REVIEWED_PREFIXES)
        if builds_native and crate.name not in catalogued and not reviewed:
            problems.append(f"{crate.label} builds or links native code and has not been reviewed in native_notices.py")
    if problems:
        raise NoticeError("; ".join(problems))


def fenced(text: str) -> str:
    return f"`````text\n{text}\n`````"


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise NoticeError(f"license source {path} cannot be read: {error}") from None


def extract(crate_root: Path, excerpt: Excerpt) -> str:
    path = crate_root / excerpt.path
    lines = read_text(path).splitlines()
    if excerpt.start is None:
        selected = lines
    else:
        first = next((i for i, line in enumerate(lines) if excerpt.start in line), None)
        last = None if first is None else next((i for i in range(first, len(lines)) if excerpt.end in lines[i]), None)
        if first is None or last is None:
            raise NoticeError(f"license source {path} no longer contains the expected license block")
        selected = lines[first : last + 1]
    text = "\n".join(line.rstrip() for line in selected).strip("\n")
    if not text:
        raise NoticeError(f"license source {path} is empty")
    return text


def render_omitted(crates: Sequence[Crate], accepted: Sequence[str], standard: set[str]) -> str:
    sections = []
    for crate in crates:
        license_ids = choose_licenses(crate.license, accepted)
        crate_root = Path(crate.metadata["manifest_path"]).parent
        files = sorted(path for path in crate_root.iterdir() if path.is_file() and LICENSE_FILE.match(path.name))
        names = ", ".join(f"`{license_id}`" for license_id in license_ids)
        section = [f"### {crate.name} {crate.version}", f"License: {crate.license} (used under {names})."]
        if files:
            for path in files:
                section.append(f"From `{path.name}`:\n\n{fenced(extract(crate_root, Excerpt(path.name)))}")
        else:
            missing = [license_id for license_id in license_ids if license_id not in standard]
            if missing:
                raise NoticeError(f"{crate.label} packages no license file and no standard text for {missing} is above")
            section.append(f"The crate packages no license file; the standard text of {names} is above.")
        sections.append("\n\n".join(section))
    if not sections:
        return ""
    preamble = (
        "## Additional Rust crates\n\n"
        "Cargo links these crates for at least one target, but cargo-about's\n"
        "dependency graph omits them."
    )
    return "\n\n".join([preamble, *sections])


def render_native(crates: Sequence[Crate]) -> str:
    sections = []
    for library in NATIVE_LIBRARIES:
        for crate in crates:
            if crate.name != library.crate:
                continue
            crate_root = Path(crate.metadata["manifest_path"]).parent
            section = [f"### {library.library} (vendored by {crate.label})"]
            for excerpt in library.sources:
                section.append(f"From `{excerpt.path}`:\n\n{fenced(extract(crate_root, excerpt))}")
            sections.append("\n\n".join(section))
    if not sections:
        return ""
    preamble = (
        "## Native libraries\n\n"
        "These libraries are compiled from C sources a Rust crate vendors and are\n"
        "linked into the binary. Their licenses are not part of the crate's license\n"
        "metadata, so they are reproduced here from the vendored sources."
    )
    return "\n\n".join([preamble, *sections])


def notices(
    markdown: str,
    tree_outputs: Iterable[str],
    metadata: Mapping[str, Any],
    accepted: Sequence[str],
) -> str:
    """The sections to append to cargo-about's `markdown` (empty if none)."""
    read_metadata(metadata)
    indexed = read_index(markdown)
    linked: set[tuple[str, str]] = set()
    for output in tree_outputs:
        linked |= read_tree(output)
    omitted_keys = sorted(linked - set(indexed))
    packages = third_party(metadata, [*indexed, *omitted_keys])
    listed = [Crate(name, version, license, packages[(name, version)]) for (name, version), license in indexed.items()]
    omitted = []
    for key in omitted_keys:
        entry = packages[key]
        if entry["license"] is None:
            raise NoticeError(f"`{key[0]} {key[1]}` is omitted by cargo-about and declares no license expression")
        omitted.append(Crate(key[0], key[1], entry["license"], entry))
    everything = sorted([*listed, *omitted], key=lambda crate: (crate.name, crate.version))
    check(everything)
    sections = [render_omitted(omitted, accepted, standard_texts(markdown)), render_native(everything)]
    body = "\n\n".join(section for section in sections if section)
    return body + "\n" if body else ""


def main(argv: Sequence[str]) -> int:
    commands = {"manifest": 3, "render": 4}
    if not argv or commands.get(argv[0]) != len(argv):
        print(
            "usage: native_notices.py manifest <checkout-root> <package>\n"
            "       native_notices.py render <checkout-root> <package> <crates-markdown>",
            file=sys.stderr,
        )
        return 2
    root, package = Path(argv[1]), argv[2]
    try:
        if argv[0] == "manifest":
            print(manifest_path(cargo_metadata(root, "--no-deps"), package))
            return 0
        targets, accepted = about_config(root)
        markdown = read_text(Path(argv[3]))
        trees = [cargo_tree(root, package, target) for target in targets]
        sys.stdout.write(notices(markdown, trees, cargo_metadata(root), accepted))
    except (NoticeError, DistributionError, OSError) as error:
        print(f"native_notices.py: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
