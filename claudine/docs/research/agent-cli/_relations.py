"""Relations gate for agent-cli research: what the contract's shape cannot express.

Usage: _relations.py <research document>...

Interim. The fleet prompt runs it in its `success` event. For a document in
the claudine package area it then runs `claudine-gen validate <slug>`, which
owns every relation between switch records (value type against
value_optional, variadic_min, and attachment; spellings two records share
where both apply; a gap wherever a fact is unknown). This script checks the
rest of the document, so neither repeats the other. The lifecycle cannot run
the generator itself: `cargo` is a denied lifecycle command and an
interpolated executable path is refused, while `python3` is a literal one.
The `research` recipe builds `claudine-gen` before the fleet starts.

Documents are read with every scalar as text, because `yes`, `no`, and `off`
are truth values to a YAML 1.1 reader.
"""
import os
import re
import subprocess
import sys

import yaml

# Each researcher's model and effort. A pair that has left the rotation stays
# while a document it wrote remains. Repeated in reasoning-level/_relations.py,
# the fleet prompts, the `research` recipe, and docs/research/_types.yaml.
ROTATION = {'opencode': [('zai-coding-plan/glm-5.3', 'provider_default')],
            'claude': [('sonnet', 'high')],
            'codex': [('gpt-6.1-sol', 'medium'), ('gpt-6-luna', 'high')]}
OSES = ('macos', 'linux', 'windows')
SECTIONS = ('## Overview', '## Installation and Binaries', '## Subcommands', '## CLI Switch Inventory',
            '## Configuration Discovery', '## Environment Variables', '## Machine Introspection',
            '## Wrapper Notes', '## Sources', '## Changelog')


def check(path):
    text = open(path).read()
    fm = yaml.load(text.split('\n---\n', 1)[0].lstrip('-\n'), Loader=yaml.BaseLoader) or {}
    out = []

    def bad(rule, msg):
        out.append((rule, msg))

    # researcher
    a, m, e = fm.get('agent'), fm.get('model'), fm.get('reasoning_effort')
    if a in ROTATION and (m, e) not in ROTATION[a]:
        bad('researcher-pairing', f'{a} must record one of {ROTATION[a]}, found {(m, e)}')
    if a == fm.get('provider'):
        bad('no-self-research', f'{a} researched its own provider')
    slug = os.path.basename(path)[:-3]
    if fm.get('provider') != slug:
        bad('provider-matches-file', f"provider {fm.get('provider')} in {slug}.md")

    # evidence: unique ids, every citation names one, every entry is cited
    evidence = fm.get('evidence') or []
    ids = [ev['id'] for ev in evidence]
    for dup in sorted({i for i in ids if ids.count(i) > 1}):
        bad('unique', f'evidence `{dup}` appears twice')
    methods = {ev['id']: ev['method'] for ev in evidence}
    switches = fm.get('cli_switches') or []
    cited = set()
    for sw in switches:
        refs = sw.get('evidence_ids') or []
        cited.update(refs)
        for ref in refs:
            if ref not in methods:
                bad('evidence-exists', f"cli_switches/{sw['flag']}: `{ref}` names no evidence entry")
        if sw.get('value_type') != 'unknown':
            if not refs:
                bad('finding-has-evidence', f"cli_switches/{sw['flag']}: value_type {sw.get('value_type')} cites no evidence")
            elif all(methods.get(ref) == 'inference' for ref in refs):
                bad('not-inference-alone', f"cli_switches/{sw['flag']}: value_type rests on inference alone")
    for unused in sorted(set(ids) - cited):
        bad('advisory:evidence-used', f'evidence `{unused}` supports no switch')

    # versions: what was examined is what the evidence describes
    examined = set(fm.get('versions_examined') or [])
    for ev in evidence:
        if ev['version'] != 'unknown' and ev['version'] not in examined:
            bad('evidence-version-examined', f"evidence `{ev['id']}` describes {ev['version']}, which versions_examined does not list")

    # subcommands are normalized, unique paths, and command scopes name one
    names = []
    for sub in fm.get('subcommands') or []:
        name = sub['name']
        if name != ' '.join(name.split()):
            bad('subcommand-path', f'subcommand `{name}` must separate its words with exactly one space')
        names.append(' '.join(name.split()))
    for dup in sorted({n for n in names if names.count(n) > 1}):
        bad('unique', f'subcommand `{dup}` appears twice')
    for sw in switches:
        for scope in sw.get('invocation_scope') or []:
            command = scope.get('command')
            if scope.get('applies_to') == 'command' and command and ' '.join(command) not in names:
                bad('scope-names-subcommand', f"cli_switches/{sw['flag']}: command path `{' '.join(command)}` is not a listed subcommand")

    # one binary record for each operating system
    seen = [b['os'] for b in fm.get('binaries') or []]
    for os_name in OSES:
        if seen.count(os_name) != 1:
            bad('binary-per-os', f'binaries has {seen.count(os_name)} records for {os_name}; expected exactly one')

    # body sections
    body = text.split('\n---\n', 1)[1] if '\n---\n' in text else ''
    for heading in SECTIONS:
        if not re.search(rf'(?m)^{re.escape(heading)}\s*$', body):
            bad('body-sections', f'missing section `{heading}`')
    return fm, out


def generator(path):
    """Runs `claudine-gen validate` for a document inside the package area.

    Returns None for a document elsewhere, such as a test fixture.
    """
    doc = os.path.abspath(path)
    area = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(doc))))
    if not os.path.isfile(os.path.join(area, 'docs', 'providers.yaml')):
        return None
    target = os.environ.get('CARGO_TARGET_DIR') or os.path.join(os.path.dirname(area), 'target')
    exe = os.path.join(target, 'debug', 'claudine-gen' + ('.exe' if os.name == 'nt' else ''))
    if not os.path.isfile(exe):
        return False, f'claudine-gen is not built at {exe}; run `cargo build -p claudine-gen`'
    slug = os.path.basename(doc)[:-3]
    env = dict(os.environ, NO_COLOR='1')
    run = subprocess.run([exe, '--area', area, 'validate', slug], capture_output=True, text=True, env=env)
    return run.returncode == 0, (run.stdout + run.stderr).strip()


if __name__ == '__main__':
    total = 0
    for p in sys.argv[1:]:
        fm, out = check(p)
        problems = [(r, msg) for r, msg in out if not r.startswith('advisory')]
        advisories = len(out) - len(problems)
        total += len(problems)
        print(f"{os.path.basename(p):16} {fm.get('agent')}/{fm.get('model')}  switches={len(fm.get('cli_switches') or [])} "
              f"subcommands={len(fm.get('subcommands') or [])} evidence={len(fm.get('evidence') or [])}  ->  "
              f"{len(problems)} problems, {advisories} advisories")
        for r, msg in problems:
            print(f'    [{r}] {msg}')
        result = generator(p)
        if result is not None:
            accepted, report = result
            total += 0 if accepted else 1
            print(f'    [generator] {report}')
    sys.exit(1 if total else 0)
