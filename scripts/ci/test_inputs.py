"""Which workspace code reads a given repository file, derived from source.

The planner selects packages by *source* (`SOURCE_SUFFIXES`), so a change to a
file that code reads rather than compiles — a Markdown document a test asserts
the wording of, a YAML schema embedded with `include_str!`, a fixture — selected
nothing, and such a test could go red on `main` and stay red
(`2026-09-22-test-input-blind-spot`). This module answers the one question the
planner was missing: for a changed or removed non-source path, which code names
it, and from where.

Two reference forms are recognized, and nothing else:

- **Embedded** — `include_str!` / `include_bytes!` of a literal (optionally
  `concat!(env!("CARGO_MANIFEST_DIR"), "…")`). Outside test code the file is
  compiled into the product, so it IS source of that package.
- **Named in test code** — a string literal, or a chain of `.join("…")`
  literals, that resolves (as written, or against the package's manifest
  directory) to the path or to a directory containing it. Only literals inside
  test context count: an integration-test target, a module declared under
  `#[cfg(test)]`, or a `#[test]` function.

Files are found the way rustc finds them — by walking `mod` declarations from
each Cargo target root — so the nextest identity of every reference is exact:
a literal inside a test function names that test, and one in a helper names the
module it lives in. A file no target reaches is not compiled and is not read.

## Notes

This is a static over-approximation, and deliberately so. A literal that
coincidentally equals a changed tracked path selects a test that did not need
to run; a path assembled at run time from non-literal parts is not seen at all.
The first costs one narrowed cell, the second is the status quo.
"""

from __future__ import annotations

import bisect
import posixpath
import re
from dataclasses import dataclass, field, replace
from typing import Callable, Iterable

#: Test-name markers the L1 tier excludes (`just _tier_filter L1`). A test
#: carrying one is not an L1 test, so a narrowed L1 cell must not name it: the
#: intersection would select nothing.
NON_L1_MARKERS = ("level2_", "level3_", "browser_", "real_", "slow_")

# One pass over the characters that change the scanner's state. Everything
# between two matches is header text. Order matters: comments and every string
# form before the single-character punctuation they may contain.
_TOKEN = re.compile(
    r"""
      (?P<line_comment>//[^\n]*)
    | (?P<block_comment>/\*.*?\*/)
    | (?P<raw>b?r(?P<hashes>\#*)"(?P<raw_body>.*?)"(?P=hashes))
    | (?P<string>b?"(?P<body>(?:[^"\\]|\\.)*)")
    | (?P<char>b?'(?:\\(?:u\{[0-9a-fA-F]{1,6}\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])')
    | (?P<open>\{)
    | (?P<close>\})
    | (?P<semi>;)
    """,
    re.VERBOSE | re.DOTALL,
)

_TEST_ATTRIBUTE = re.compile(
    r"#\[\s*(?:[A-Za-z_][\w]*::)*(?:test|rstest|test_case|test_log::test)\b"
)
_CFG_TEST = re.compile(r"#\[\s*cfg\s*\((?![^\]]*\bnot\s*\(\s*test\b)[^\]]*\btest\b")
_FN = re.compile(r"\bfn\s+(r#)?([A-Za-z_]\w*)")
_MOD_BLOCK = re.compile(r"\bmod\s+(r#)?([A-Za-z_]\w*)\s*$")
_MOD_DECL = re.compile(r"\bmod\s+(r#)?([A-Za-z_]\w*)\s*$")
_PATH_ATTRIBUTE = re.compile(r'#\[\s*path\s*=\s*"([^"]+)"\s*\]')
_INCLUDE = re.compile(r"\binclude_(?:str|bytes)\s*!\s*\(\s*$")
_INCLUDE_CONCAT = re.compile(
    r'\binclude_(?:str|bytes)\s*!\s*\(\s*concat\s*!\s*\(\s*env\s*!\s*\(\s*$'
)
_LITERAL = re.compile(r'"([^"\\\n]{1,300})"')
_MOD_LINE = re.compile(
    r"^(?P<indent>[ \t]*)(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+(?:r#)?(?P<name>[A-Za-z_]\w*)\s*;",
    re.MULTILINE,
)
# How much header text before a `{` or `;` the attribute and item patterns read.
# Headers are normally a few lines; the bound keeps a statement that carries a
# large literal from being searched again at every token.
_HEADER_TAIL = 2000
_PLAUSIBLE = re.compile(r"[A-Za-z0-9_.@+\-][A-Za-z0-9_./@+\-]*")
# A rooted path expression: a root, then any `.parent()` / `.join("…")` steps.
# The root is the manifest directory (`manifest_dir!()` or
# `CARGO_MANIFEST_DIR`), a repository-root helper by one of the names in
# `REPOSITORY_ROOTS`, or a name this file bound to a rooted expression
# (`let root = repo_root();`, `fn docs() -> PathBuf { manifest_dir!().join("docs") }`).
_HEAD = (
    r'(?P<manifest>(?:\w+::)*manifest_dir!\s*\(\s*\)'
    r'|(?:std::path::)?(?:Path::new|PathBuf::from)\s*\(\s*env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*\)'
    r'|env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\))'
    r"|(?P<dot>\.\s*)?(?:\w+::)*(?P<name>[A-Za-z_]\w*)(?P<call>\s*\(\s*\))?"
)
_STEPS = (
    r"(?P<steps>(?:\s*\.\s*(?:"
    r'parent\s*\(\s*\)(?:\s*\.\s*(?:unwrap\s*\(\s*\)|expect\s*\(\s*"[^"]*"\s*\)))?'
    r'|join\s*\(\s*"[^"]*"\s*\)'
    r"|to_path_buf\s*\(\s*\)"
    r"))*)"
)
_STEP = re.compile(r'(?P<parent>parent)\s*\(|join\s*\(\s*"(?P<join>[^"]*)"')
_ANY_ROOT = re.compile(rf"(?:{_HEAD})\s*(?=[.;)])")
_JOINED = re.compile(rf"(?:{_HEAD}){_STEPS}\s*\.\s*join\s*\(\s*$")
_LET_BINDING = re.compile(
    rf"\blet\s+(?:mut\s+)?(?P<bound>[A-Za-z_]\w*)\s*(?::[^=;]+)?=\s*(?:{_HEAD}){_STEPS}\s*;"
)
_FN_BINDING = re.compile(
    rf"\bfn\s+(?P<bound>[A-Za-z_]\w*)\s*\(\s*\)\s*->\s*(?:std::path::)?PathBuf\s*\{{\s*(?:{_HEAD}){_STEPS}\s*\}}"
)
# The argument of these is relative to the working directory, or is not a path
# at all; a literal there is mock data far more often than a read.
_PATH_CONSTRUCTOR = re.compile(r"(?:Path::new|PathBuf::from)\s*\(\s*$")
# Helper names that return the repository root wherever this workspace defines
# them. Anything else must be bound in the same file to count as a root.
REPOSITORY_ROOTS = frozenset({"repo_root", "repository_root", "workspace_root"})


@dataclass(frozen=True)
class Target:
    """One Cargo target whose `mod` tree is walked.

    `binary_id` is nextest's identity for the target's test binary:
    `<package>` for a library, `<package>::bin/<name>` for a binary, and
    `<package>::<name>` for an integration test. `built_for_l1` is false when
    the target's `required-features` are not all enabled by the package's CI
    feature contract, so the L1 cell never compiles it.
    """

    package: str
    kind: str
    name: str
    root: str
    manifest_dir: str
    built_for_l1: bool = True

    @property
    def binary_id(self) -> str:
        if self.kind == "lib":
            return self.package
        if self.kind == "bin":
            return f"{self.package}::bin/{self.name}"
        return f"{self.package}::{self.name}"


@dataclass(frozen=True)
class Reference:
    """One place in compiled code that names a repository path.

    `path` is the resolved repository path the literal names — the changed path
    itself, or a directory containing it. `unit` is the nextest filterset that
    selects the test reading it, and is `None` exactly when `product` is true:
    an embedded file outside test code has no test to narrow to.
    """

    path: str
    package: str
    source: str
    line: int
    embedded: bool
    product: bool
    unit: str | None


@dataclass
class _Scope:
    kind: str
    name: str = ""
    test: bool = False


@dataclass
class _File:
    target: Target
    path: str
    module: tuple[str, ...]
    test: bool
    # Where this file's own child modules live: the file's directory for a
    # crate root or a `mod.rs`, otherwise `<dir>/<stem>`.
    module_dir: str
    children: list[tuple[str, tuple[str, ...], bool, str | None]] = field(default_factory=list)


def normalize(path: str) -> str | None:
    """A repository-relative path with `.` and `..` folded, or `None` if it escapes."""
    parts: list[str] = []
    for component in path.replace("\\", "/").split("/"):
        if component in ("", "."):
            continue
        if component == "..":
            if not parts:
                return None
            parts.pop()
        else:
            parts.append(component)
    return "/".join(parts) if parts else None


def _l1_features(package: dict) -> set[str] | None:
    """The features a package's L1 cell compiles with, or `None` for all of them.

    Mirrors `affected_scope.feature_args`, which reads the same
    `[package.metadata.ci.tests]` `features` / `all-features` keys: the
    requested features plus `default`, closed over the package's feature table.
    """
    tests = (((package.get("metadata") or {}).get("ci") or {}).get("tests")) or {}
    if tests.get("all-features"):
        return None
    table = package.get("features") or {}
    enabled: set[str] = set()
    pending = ["default", *tests.get("features", [])]
    while pending:
        feature = pending.pop()
        if feature in enabled or feature.startswith("dep:"):
            continue
        # `dep/feature` enables the optional dependency's implicit feature
        # `dep`; `dep?/feature` does not.
        name = feature.split("/", 1)[0]
        if "/" in feature and name.endswith("?"):
            continue
        if name in enabled:
            continue
        enabled.add(name)
        pending.extend(table.get(name, []))
    return enabled


def targets_from_metadata(
    packages: Iterable[dict], root: str
) -> list[Target]:
    """The lib, bin, and test targets of workspace packages.

    Benches and examples are left out on purpose: no L1 cell runs them, so a
    reference found there could only schedule a cell that selects nothing. A
    target whose `required-features` the L1 cell does not enable is kept, with
    `built_for_l1` false: its embeds are still product source.
    """
    prefix = root.replace("\\", "/").rstrip("/") + "/"
    targets: list[Target] = []
    for package in packages:
        manifest = package["manifest_path"].replace("\\", "/")
        manifest_dir = posixpath.dirname(manifest).removeprefix(prefix)
        features = _l1_features(package)
        for target in package.get("targets", []):
            kinds = set(target.get("kind", []))
            if kinds & {"lib", "rlib", "proc-macro", "dylib", "cdylib", "staticlib"}:
                kind = "lib"
            elif "bin" in kinds:
                kind = "bin"
            elif "test" in kinds:
                kind = "test"
            else:
                continue
            source = target.get("src_path", "").replace("\\", "/")
            if not source.startswith(prefix):
                continue
            targets.append(
                Target(
                    package=package["name"],
                    kind=kind,
                    name=target["name"],
                    root=source.removeprefix(prefix),
                    manifest_dir=manifest_dir,
                    built_for_l1=features is None
                    or set(target.get("required-features") or ()) <= features,
                )
            )
    return targets


def _module_dir(path: str, is_root: bool) -> str:
    directory, name = posixpath.split(path)
    if is_root or name == "mod.rs":
        return directory
    return posixpath.join(directory, name.removesuffix(".rs"))


def _is_l1(name: str, include_slow: bool) -> bool:
    markers = tuple(
        marker for marker in NON_L1_MARKERS if not (include_slow and marker == "slow_")
    )
    return not any(segment.startswith(markers) for segment in name.split("::"))


def _unit(
    target: Target, module: tuple[str, ...], test_fn: str | None, include_slow: bool
) -> str | None:
    """The nextest filterset for a reference at `module` (inside `test_fn`).

    Inside a test function the unit is that test, or `None` when its path
    carries a marker the L1 tier excludes; a binary's name says nothing about
    its tier (`biscuit-tui-cli::level2` holds L1 tests). Anywhere else the
    reference sits in a helper whose callers are unknown: a `pub` item, or a
    private one wrapped by a public one, can be called from any module of the
    binary. So the unit is the whole binary, and `scan` drops it only when the
    binary holds no L1 test. The unit never needs to exclude non-L1 tests
    itself: the cell intersects it with the tier expression
    (`BISCUIT_TEST_NARROW`).
    """
    if not target.built_for_l1:
        return None
    binary = f"binary_id({target.binary_id})"
    if test_fn is None:
        return binary
    name = "::".join((*module, test_fn))
    if not _is_l1(name, include_slow):
        return None
    return f"{binary} & test(={name})"


def _depth(literal: str) -> int:
    return sum(1 for part in literal.split("/") if part not in ("", ".", ".."))


def _rooted_base(
    match: re.Match[str], manifest_dir: str, bound: dict[str, str]
) -> str | None:
    """The repository path a rooted expression evaluates to, `""` being the root.

    `None` when the head is not a root this scanner knows: a method call
    (`fixture.cwd()`), an unbound name, or steps that climb out of the tree.
    """
    if match.group("manifest"):
        base = manifest_dir
    elif match.group("dot"):
        return None
    elif match.group("call") and match.group("name") in REPOSITORY_ROOTS:
        base = ""
    else:
        key = match.group("name") + ("()" if match.group("call") else "")
        if key not in bound:
            return None
        base = bound[key]
    for step in _STEP.finditer(match.group("steps")):
        if step.group("parent"):
            if not base:
                return None
            base = posixpath.dirname(base)
        else:
            joined = normalize(posixpath.join(base, step.group("join")))
            if joined is None:
                return None
            base = joined
    return base


def _bindings(text: str, manifest_dir: str) -> dict[str, str]:
    """Names this file binds to a rooted expression, in declaration order.

    A `let` binds `name` and a zero-argument `fn … -> PathBuf` binds `name()`.
    Scoping is ignored: a name bound anywhere in the file counts everywhere in
    it, which over-selects only when one file reuses a root's name for a
    tempdir.
    """
    bound: dict[str, str] = {}
    found = [(match.start(), "", match) for match in _LET_BINDING.finditer(text)]
    found += [(match.start(), "()", match) for match in _FN_BINDING.finditer(text)]
    for _, suffix, match in sorted(found, key=lambda entry: entry[0]):
        base = _rooted_base(match, manifest_dir, bound)
        if base is not None:
            bound[match.group("bound") + suffix] = base
    return bound


class _Matcher:
    """Membership tests for one candidate set, built once per scan."""

    def __init__(self, candidates: Iterable[str]) -> None:
        self.exact = {path for path in (normalize(entry) for entry in candidates) if path}
        self.directories: dict[str, set[str]] = {}
        for path in self.exact:
            parts = path.split("/")
            # A single top-level directory is too coarse to be a reference:
            # `docs` or `scripts` named in a test says nothing about which file
            # under it the test reads.
            for depth in range(2, len(parts)):
                self.directories.setdefault("/".join(parts[:depth]), set()).add(path)
        # The last component any matching literal must end with: a candidate's
        # file name, or a directory containing one.
        self.needles = {path.rsplit("/", 1)[-1] for path in self.exact} | {
            directory.rsplit("/", 1)[-1] for directory in self.directories
        }

    def may_match(self, literal: str) -> bool:
        return literal.rstrip("/").rsplit("/", 1)[-1] in self.needles

    def matches(self, resolved: str, manifest_dir: str, *, directories: bool) -> list[str]:
        """The candidate paths `resolved` names, as a file or as a containing directory.

        Directory containment counts only for a root-anchored join: a relative
        `.claude/skills` is far more often a tempdir fixture than a read of the
        repository's own skills.
        """
        hits = [resolved] if resolved in self.exact else []
        contained = self.directories.get(resolved) if directories else None
        # A directory that contains the package itself (`darkmatter`,
        # `darkmatter/lib`) is where the package lives, not an input it reads.
        if contained and not (
            manifest_dir == resolved or manifest_dir.startswith(resolved + "/")
        ):
            hits.extend(sorted(contained))
        return hits


def _scan_file(
    text: str,
    unit: _File,
    matcher: _Matcher,
    out: list[Reference],
    include_slow: bool = False,
    *,
    wide: list[Reference] | None = None,
    tests: list[str] | None = None,
) -> None:
    """Record every reference in one file and its `mod` declarations.

    A reference whose unit is binary-wide rather than one test is also added to
    `wide`; `tests` collects the full name of every test function the file
    defines.
    """
    target = unit.target
    lines = [0]
    lines.extend(match.end() for match in re.finditer("\n", text))
    stack: list[_Scope] = []
    boundary = 0
    bound: dict[str, str] | None = None
    anchored: bool | None = None

    def roots() -> dict[str, str]:
        nonlocal bound
        if bound is None:
            bound = _bindings(text, target.manifest_dir)
        return bound

    def anchored_file() -> bool:
        """Whether the file reads through a root anywhere.

        A multi-component literal that is not itself joined onto a root still
        names a repository file in such a file — a table of document paths
        walked by `root.join(entry)`. A file that never touches a root is
        building fixtures or mock data, where `.github/workflows/ci.yml` is a
        string, not a read.
        """
        nonlocal anchored
        if anchored is None:
            anchored = bool(roots()) or any(
                match.group("manifest")
                or (
                    match.group("call")
                    and match.group("name") in REPOSITORY_ROOTS
                    and not match.group("dot")
                )
                for match in _ANY_ROOT.finditer(text)
            )
        return anchored

    def line_of(offset: int) -> int:
        return bisect.bisect_right(lines, offset)

    def context() -> tuple[bool, tuple[str, ...], str | None]:
        in_test = unit.test
        module = list(unit.module)
        test_fn = None
        for scope in stack:
            if scope.test:
                in_test = True
            if scope.kind == "mod":
                module.append(scope.name)
            elif scope.kind == "test_fn" and test_fn is None:
                test_fn = scope.name
        return in_test, tuple(module), test_fn

    def record(literal: str, start: int, header: str) -> None:
        if len(literal) > 300 or not matcher.may_match(literal):
            return
        if not _PLAUSIBLE.fullmatch(literal):
            return
        tail = header[-_HEADER_TAIL:]
        concat = bool(_INCLUDE_CONCAT.search(tail))
        embedded = concat or bool(_INCLUDE.search(tail))
        in_test, module, test_fn = context()
        if not embedded and not in_test:
            return
        directories = False
        if concat:
            resolved = {normalize(posixpath.join(target.manifest_dir, literal))}
        elif embedded:
            resolved = {normalize(posixpath.join(posixpath.dirname(unit.path), literal))}
        elif tail.rstrip().endswith("("):
            joined = _JOINED.search(tail)
            if joined is not None:
                base = _rooted_base(joined, target.manifest_dir, roots())
                if base is None:
                    return
                resolved = {normalize(posixpath.join(base, literal))}
                directories = True
            elif _PATH_CONSTRUCTOR.search(tail) or tail.rstrip().endswith(".join("):
                return
            else:
                resolved = free(literal)
        else:
            resolved = free(literal)
        product = embedded and not in_test
        for path in sorted(entry for entry in resolved if entry):
            for hit in matcher.matches(path, target.manifest_dir, directories=directories):
                reference = Reference(
                    path=hit,
                    package=target.package,
                    source=unit.path,
                    line=line_of(start),
                    embedded=embedded,
                    product=product,
                    unit=None if product else _unit(target, module, test_fn, include_slow),
                )
                out.append(reference)
                if wide is not None and test_fn is None and reference.unit is not None:
                    wide.append(reference)

    def free(literal: str) -> set[str | None]:
        if _depth(literal) < 2 or not anchored_file():
            return set()
        return {normalize(literal), normalize(posixpath.join(target.manifest_dir, literal))}

    for match in _TOKEN.finditer(text):
        kind = match.lastgroup
        start = match.start()
        if kind in ("line_comment", "block_comment", "char"):
            continue
        if kind in ("string", "raw"):
            header = text[boundary:start]
            body = match.group("body") if kind == "string" else match.group("raw_body")
            if body is not None and not match.group(0).startswith("b"):
                record(body, start, header)
            continue
        header = text[max(boundary, start - _HEADER_TAIL):start]
        if kind == "open":
            scope = _Scope("block")
            module_match = _MOD_BLOCK.search(header)
            function = _FN.search(header)
            if module_match:
                scope = _Scope("mod", module_match.group(2))
            elif function:
                is_test = bool(_TEST_ATTRIBUTE.search(header))
                scope = _Scope("test_fn" if is_test else "fn", function.group(2), is_test)
                if is_test and tests is not None:
                    _, module, enclosing = context()
                    if enclosing is None:
                        tests.append("::".join((*module, function.group(2))))
            if _CFG_TEST.search(header):
                scope.test = True
            stack.append(scope)
        elif kind == "close":
            if stack:
                stack.pop()
        elif kind == "semi":
            declaration = _MOD_DECL.search(header)
            if declaration:
                in_test, module, _ = context()
                path_attribute = _PATH_ATTRIBUTE.search(header)
                unit.children.append(
                    (
                        declaration.group(2),
                        module,
                        in_test or bool(_CFG_TEST.search(header)),
                        path_attribute.group(1) if path_attribute else None,
                    )
                )
        boundary = match.end()


def _declarations(text: str, unit: _File) -> None:
    """The top-level `mod name;` declarations of a file, with their attributes.

    Only valid for a file whose declarations are all at column zero; `scan`
    routes every other file through the full scanner.
    """
    for match in _MOD_LINE.finditer(text):
        attributes: list[str] = []
        cursor = match.start()
        while cursor > 0:
            line_start = text.rfind("\n", 0, cursor - 1) + 1
            line = text[line_start:cursor].strip()
            if not line.startswith("#["):
                break
            attributes.append(line)
            cursor = line_start
        header = "\n".join(attributes)
        path_attribute = _PATH_ATTRIBUTE.search(header)
        unit.children.append(
            (
                match.group("name"),
                unit.module,
                unit.test or bool(_CFG_TEST.search(header)),
                path_attribute.group(1) if path_attribute else None,
            )
        )


def scan(
    targets: Iterable[Target],
    read: Callable[[str], str | None],
    candidates: Iterable[str],
    include_slow: frozenset[str] = frozenset(),
) -> list[Reference]:
    """Every reference, in compiled code, to one of `candidates`.

    `include_slow` names the packages whose L1 contract keeps `slow_` tests
    (`l1-include-slow`); everywhere else a `slow_` test is not an L1 test.

    `read` returns a repository file's text, or `None` when it does not exist
    (a declared module whose file is missing is simply not walked). A file
    reached from several targets — a shared `tests/common` helper — is reported
    once per target, because each target's binary reads it.

    A reference outside any test function names its whole binary, and gets no
    unit when that binary holds no L1 test. Only those targets' files are
    tokenized a second time.
    """
    matcher = _Matcher(candidates)
    if not matcher.exact:
        return []
    references: list[Reference] = []
    cache: dict[str, str | None] = {}

    def cached(path: str) -> str | None:
        if path not in cache:
            cache[path] = read(path)
        return cache[path]

    hot: dict[str, bool] = {}

    def needs_tokens(path: str, text: str) -> bool:
        """Whether a file needs the full scanner rather than its declarations alone.

        Full scanning is what makes module paths and test context exact, and
        costs ~2 µs per token over ~55 MB of workspace Rust. Only two kinds of
        file need it: one with a literal that could name a candidate, and one
        declaring a module file from inside an inline module, whose path the
        line-level pattern cannot know.
        """
        if path not in hot:
            hot[path] = any(
                matcher.may_match(literal) for literal in _LITERAL.findall(text)
            ) or any(match.group("indent") for match in _MOD_LINE.finditer(text))
        return hot[path]

    wide: dict[Target, list[Reference]] = {}
    walked: dict[Target, list[_File]] = {}

    for target in targets:
        pending = [
            _File(
                target=target,
                path=target.root,
                module=(),
                test=target.kind == "test",
                module_dir=_module_dir(target.root, True),
            )
        ]
        visited: set[str] = set()
        while pending:
            unit = pending.pop()
            if unit.path in visited:
                continue
            visited.add(unit.path)
            text = cached(unit.path)
            if text is None:
                continue
            walked.setdefault(target, []).append(unit)
            if needs_tokens(unit.path, text):
                _scan_file(
                    text,
                    unit,
                    matcher,
                    references,
                    target.package in include_slow,
                    wide=wide.setdefault(target, []),
                )
            else:
                _declarations(text, unit)
            for name, module, is_test, path_attribute in unit.children:
                if path_attribute is not None:
                    candidates_for_child = [
                        normalize(posixpath.join(posixpath.dirname(unit.path), path_attribute))
                    ]
                else:
                    base = posixpath.join(unit.module_dir, *module[len(unit.module):])
                    candidates_for_child = [
                        normalize(posixpath.join(base, f"{name}.rs")),
                        normalize(posixpath.join(base, name, "mod.rs")),
                    ]
                for child in candidates_for_child:
                    if child and cached(child) is not None:
                        pending.append(
                            _File(
                                target=target,
                                path=child,
                                module=(*module, name),
                                test=is_test,
                                module_dir=_module_dir(child, False),
                            )
                        )
                        break

    unreachable = {
        reference
        for target, entries in wide.items()
        if entries
        and not _has_l1_test(walked.get(target, []), cached, target.package in include_slow)
        for reference in entries
    }
    references = [
        replace(entry, unit=None) if entry in unreachable else entry for entry in references
    ]
    return sorted(
        set(references),
        key=lambda entry: (entry.path, entry.package, entry.source, entry.line, entry.unit or ""),
    )


def _has_l1_test(
    walked: list[_File], cached: Callable[[str], str | None], include_slow: bool
) -> bool:
    """Whether any test in one binary's walked files belongs to the L1 tier.

    A binary-wide unit in a binary holding only `level2_*` tests would be
    emptied by the tier expression, and the narrowed cell, which runs with
    `--no-tests=fail`, would go red having found nothing to run.
    """
    silent = _Matcher(())
    for unit in walked:
        found: list[str] = []
        fresh = _File(unit.target, unit.path, unit.module, unit.test, unit.module_dir)
        _scan_file(cached(unit.path) or "", fresh, silent, [], tests=found)
        if any(_is_l1(name, include_slow) for name in found):
            return True
    return False
