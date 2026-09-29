"""Relations gate for reasoning-level research: what the contract's shape cannot express.

Usage: _relations.py <research document>...

Interim. The fleet prompt runs it in its `success` event. It is to be replaced
by one checker that reads each topic's relations from a declaration.

Documents are read with every scalar as text, because `yes`, `no`, and `off`
are truth values to a YAML 1.1 reader.
"""
import sys, yaml, re, json, glob, os

ROTATION = {'opencode': ('zai-coding-plan/glm-5.3', 'provider_default'),
            'claude': ('sonnet', 'high'), 'codex': ('gpt-6-luna', 'high')}
OWN = {'claude': 'claude', 'codex': 'codex', 'opencode': 'opencode'}
ARG_KINDS = {'launch_flag', 'config_override_flag', 'model_suffix'}

def check(path):
    fm = yaml.load(open(path).read().split('\n---\n', 1)[0].lstrip('-\n'), Loader=yaml.BaseLoader)
    out = []
    def bad(rule, msg): out.append((rule, msg))
    ev = {e['id'] for e in fm.get('evidence', [])}
    controls = {c['id']: c for c in fm.get('controls', [])}
    natives = [l['native'] for l in fm.get('levels', [])]

    # researcher
    a, m, e = fm.get('agent'), fm.get('model'), fm.get('reasoning_effort')
    if a in ROTATION and (m, e) != ROTATION[a]:
        bad('researcher-pairing', f'{a} must record {ROTATION[a]}, found {(m, e)}')
    if OWN.get(a) == fm.get('provider'):
        bad('no-self-research', f'{a} researched its own provider')
    slug = os.path.basename(path)[:-3]
    if fm.get('provider') != slug:
        bad('provider-matches-file', f"provider {fm.get('provider')} in {slug}.md")

    # every evidence reference exists; every evidence entry is used
    used = set()
    def refs(node, where):
        if isinstance(node, dict):
            for k, v in node.items():
                if k == 'evidence_ids':
                    for i in v or []:
                        used.add(i)
                        if i not in ev: bad('evidence-exists', f'{where}: `{i}` names no evidence entry')
                else: refs(v, f'{where}/{k}')
        elif isinstance(node, list):
            for i, v in enumerate(node): refs(v, f'{where}/{i}')
    refs({k: v for k, v in fm.items() if k != 'evidence'}, '')
    for i in sorted(ev - used): bad('advisory:evidence-used', f'evidence `{i}` supports no finding')

    # a finding needs evidence unless it is unknown
    def needs(node, where, unknown):
        if not node.get('evidence_ids') and not unknown:
            bad('finding-has-evidence', f'{where} states a finding with no evidence')
    for l in fm.get('levels', []): needs(l, f"levels/{l['native']}", False)
    for c in fm.get('controls', []): needs(c, f"controls/{c['id']}", False)
    for mo in fm.get('models', []): needs(mo, f"models/{mo['model']}", False)
    dl = fm.get('default_level', {}); needs(dl, 'default_level', dl.get('decided_by') == 'unknown' and not dl.get('native'))
    il = fm.get('invalid_level', {}); needs(il, 'invalid_level', il.get('behavior') == 'unknown')
    rp = fm.get('reporting', {}); needs(rp, 'reporting', rp.get('source') in ('unknown',))
    ro = fm.get('reasoning_output', {}); needs(ro, 'reasoning_output', ro.get('reaches_caller') == 'unknown')

    # inference alone establishes nothing
    methods = {e['id']: e['method'] for e in fm.get('evidence', [])}
    def only_inference(node, where):
        ids = node.get('evidence_ids') or []
        if ids and all(methods.get(i) == 'inference' for i in ids):
            bad('not-inference-alone', f'{where} rests on inference alone')
    for l in fm.get('levels', []): only_inference(l, f"levels/{l['native']}")
    for c in fm.get('controls', []): only_inference(c, f"controls/{c['id']}")

    # support agrees with the rest
    sup = fm.get('support')
    if sup == 'none' and (natives or controls): bad('support-consistent', 'support is none but levels or controls are listed')
    if sup in ('every_model', 'some_models') and not natives: bad('support-consistent', f'support is {sup} but no level is listed')
    if sup in ('every_model', 'some_models') and not controls: bad('support-consistent', f'support is {sup} but no control is listed')
    if sup == 'every_model' and any(not mo['accepts'] for mo in fm.get('models', [])):
        bad('support-consistent', 'support is every_model but a model accepts no level')

    # identifiers are unique
    for name, items, key in (('levels', fm.get('levels', []), 'native'), ('controls', fm.get('controls', []), 'id'),
                             ('models', fm.get('models', []), 'model'), ('evidence', fm.get('evidence', []), 'id')):
        seen = set()
        for it in items:
            if it[key] in seen: bad('unique', f'{name}: `{it[key]}` appears twice')
            seen.add(it[key])

    # precedence names controls, once each
    prec = fm.get('precedence', [])
    for p in prec:
        if p not in controls: bad('precedence-names-controls', f'precedence: `{p}` names no control')
    if len(set(prec)) != len(prec): bad('precedence-names-controls', 'precedence lists a control twice')
    if len(controls) > 1 and not prec and not any(g for g in fm.get('gaps', []) if g.get('area') == 'precedence'):
        bad('precedence-or-gap', 'several controls, no precedence, and no gap explaining why')

    # arguments: only for command-line kinds, and they carry the placeholder
    for c in controls.values():
        args = c.get('arguments') or []
        if args and c['kind'] not in ARG_KINDS:
            bad('arguments-for-command-line-kinds', f"controls/{c['id']}: kind {c['kind']} is not a command-line argument but has arguments {args}")
        if args and c['value'] != 'on_or_off' and not any(re.search(r'<[a-z_]+>', x) for x in args):
            bad('arguments-carry-placeholder', f"controls/{c['id']}: arguments {args} have no placeholder for the value")
        if c['kind'] in ('launch_flag', 'config_override_flag') and not args:
            bad('launch-controls-have-arguments', f"controls/{c['id']}: a {c['kind']} without arguments")
        if c['kind'] == 'session_command' and 'non_interactive' in (c.get('launch_modes') or []) and c['changes_running_session'] == 'no':
            pass

    # levels a model accepts, and defaults, are listed levels
    for mo in fm.get('models', []):
        for t in mo['accepts']:
            if t not in natives: bad('model-levels-are-levels', f"models/{mo['model']}: `{t}` is not a listed level")
        if mo.get('default') and mo['default'] not in natives and mo['default'] not in ('on', 'off'): bad('model-levels-are-levels', f"models/{mo['model']}: default `{mo['default']}` is not a listed level")
    if dl.get('native') and dl['native'] not in natives: bad('default-is-a-level', f"default_level `{dl['native']}` is not a listed level")

    # normalized scale follows the listed order
    order = ['off', 'minimal', 'low', 'medium', 'high', 'very_high', 'maximum']
    scale = [l for l in fm.get('levels', []) if l['normalized'] != 'outside_scale']
    ranks = [order.index(l['normalized']) for l in scale]
    seen_mode = False
    for l in fm.get('levels', []):
        if l['normalized'] == 'outside_scale': seen_mode = True
        elif seen_mode: bad('modes-after-scale', f"level `{l['native']}` follows a mode that is outside the scale")
    if ranks != sorted(ranks): bad('levels-weakest-first', f"levels are not weakest first: {[(l['native'], l['normalized']) for l in fm.get('levels', [])]}")

    # reporting and reasoning output
    if rp.get('source') == 'status_command' and not rp.get('command'):
        bad('reporting-has-locator', 'reporting source is status_command with no command')
    if rp.get('source') != 'status_command' and rp.get('command'):
        bad('reporting-has-locator', f"reporting source is {rp.get('source')} but a command is given")
    if rp.get('source') not in ('nowhere', 'unknown', 'status_command', None) and not rp.get('locator'):
        bad('reporting-has-locator', f"reporting source is {rp.get('source')} with no locator")
    if rp.get('source') in ('nowhere', 'unknown') and (rp.get('locator') or rp.get('field')):
        bad('reporting-has-locator', f"reporting source is {rp.get('source')} but a locator or field is given")
    if ro.get('control_id') and ro['control_id'] not in controls: bad('control-exists', f"reasoning_output: `{ro['control_id']}` names no control")
    if il.get('behavior') == 'unknown' and il.get('message'): bad('invalid-level-consistent', 'behavior is unknown but a message is recorded')

    # every unknown has a gap
    unknowns = []
    def walk(node, where):
        if isinstance(node, dict):
            for k, v in node.items(): walk(v, f'{where}/{k}' if where else k)
        elif isinstance(node, list):
            for i, v in enumerate(node):
                key = v.get('id') or v.get('native') or v.get('model') if isinstance(v, dict) else i
                walk(v, f'{where}/{key}')
        elif node == 'unknown': unknowns.append(where)
    walk({k: v for k, v in fm.items() if k not in ('evidence', 'gaps')}, '')
    if unknowns and not fm.get('gaps'): bad('unknown-has-gap', f'{len(unknowns)} unknown values and no gaps: {unknowns[:6]}')
    names = set(controls) | set(natives) | {mo['model'] for mo in fm.get('models', [])}
    for g in fm.get('gaps', []):
        if g.get('entry') and g['entry'] not in names:
            bad('gap-entry-exists', f"gap entry `{g['entry']}` names no level, control, or model")
    areas = {g.get('area') for g in fm.get('gaps', [])}
    for where in unknowns:
        top = where.split('/')[0]
        area = {'default_level': 'default_level', 'invalid_level': 'invalid_level', 'reporting': 'reporting',
                'reasoning_output': 'reasoning_output', 'controls': 'controls', 'levels': 'levels', 'models': 'models',
                'support': 'other'}.get(top, 'other')
        if area not in areas and 'other' not in areas:
            bad('unknown-has-gap', f'`{where}` is unknown and no gap covers {area}')

    # body sections
    body = open(path).read().split('\n---\n', 1)[1]
    for h in ('## Levels', '## Choosing a Level', '## Models', '## Confirming the Level', '## Sources', '## Changelog'):
        if not re.search(rf'(?m)^{re.escape(h)}\s*$', body): bad('body-sections', f'missing section `{h}`')
    return fm, out, unknowns

if __name__ == '__main__':
    total = 0
    for p in sys.argv[1:]:
        fm, out, unknowns = check(p)
        total += len([1 for r, m in out if not r.startswith('advisory')])
        print(f"{os.path.basename(p):16} {fm.get('agent')}/{fm.get('model')}  levels={len(fm.get('levels', []))} controls={len(fm.get('controls', []))} models={len(fm.get('models', []))} evidence={len(fm.get('evidence', []))} gaps={len(fm.get('gaps', []))} unknowns={len(unknowns)}  ->  {len([1 for r, m in out if not r.startswith('advisory')])} problems, {len([1 for r, m in out if r.startswith('advisory')])} advisories")
        for r, m in out:
            if not r.startswith('advisory'): print(f"    [{r}] {m}")
    sys.exit(1 if total else 0)
