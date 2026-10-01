"""Comparison tables for reasoning-level research, produced from frontmatter alone.

Usage: reasoning_level_tables.py <directory of research documents> <roster file>

A prototype of what a refresh can report without a model. Documents are read
with every scalar as text.
"""
import os, sys, yaml

def load(path):
    text = open(path).read()
    return yaml.load(text.split('\n---\n', 1)[0].lstrip('-\n'), Loader=yaml.BaseLoader)

def code(value):
    return f'`{value}`' if value else '—'

BEHAVIOR = {'uses_default': 'runs at the default', 'fails_the_request': 'fails the request',
            'uses_nearest_level': 'uses the nearest level', 'refuses_to_start': 'refuses to start', 'unknown': 'unknown'}
SOURCE = {'session_record': 'session record', 'nowhere': 'nowhere', 'stream_event': 'stream event',
          'log_file': 'log file', 'status_command': 'a command', 'unknown': 'unknown'}
OUTPUT = {'full_text': 'in full', 'summary': 'as a summary', 'hidden': 'no', 'unknown': 'unknown'}
KIND = {'config_file_key': 'configuration key', 'environment_variable': 'environment variable',
        'session_command': 'session command', 'request_field': 'request field', 'model_suffix': 'model suffix',
        'launch_flag': 'launch flag', 'config_override_flag': 'override flag'}
LAUNCH = ('launch_flag', 'config_override_flag', 'model_suffix')

def main(directory, roster):
    entries = yaml.safe_load(open(roster))['sequence']
    docs = [(e['display_name'], load(os.path.join(directory, e['file']))) for e in entries
            if os.path.exists(os.path.join(directory, e['file']))]
    levels = ['| Provider | Version | Levels of the scale | Modes outside the scale | Default |', '| --- | --- | --- | --- | --- |']
    controls = ['| Provider | At launch | Other controls |', '| --- | --- | --- |']
    behavior = ['| Provider | An unaccepted level | Warns | Level reported in | Reasoning text reaches the caller |', '| --- | --- | --- | --- | --- |']
    runs = ['| Provider | Researcher | Evidence entries | From live tests | Gaps |', '| --- | --- | --- | --- | --- |']
    for name, d in docs:
        scale = [l['native'] for l in d['levels'] if l['normalized'] != 'outside_scale']
        modes = [l['native'] for l in d['levels'] if l['normalized'] == 'outside_scale']
        levels.append(f"| {name} | {', '.join(d['versions_examined'])} | {', '.join(map(code, scale)) or '—'} | {', '.join(map(code, modes)) or '—'} | {code(d['default_level'].get('native'))} |")
        launch, seen, other = [], set(), {}
        for c in d['controls']:
            if c['kind'] in LAUNCH and c['arguments']:
                shown = ' '.join(c['arguments'])
                if shown not in seen:
                    seen.add(shown); launch.append(code(shown))
            else:
                other[c['kind']] = other.get(c['kind'], 0) + 1
        rest = ', '.join(f"{n} {KIND[k]}{'s' if n > 1 else ''}" for k, n in other.items())
        controls.append(f"| {name} | {'<br>'.join(launch) or 'none'} | {rest or 'none'} |")
        invalid = d['invalid_level']
        behavior.append(f"| {name} | {BEHAVIOR[invalid['behavior']]} | {invalid['warns']} | {SOURCE[d['reporting']['source']]} | {OUTPUT[d['reasoning_output']['reaches_caller']]} |")
        live = sum(1 for e in d['evidence'] if e['method'] == 'disposable_test')
        runs.append(f"| {name} | {d['agent']}, `{d['model']}` | {len(d['evidence'])} | {live} | {len(d['gaps'])} |")
    for title, table in (('Levels', levels), ('How a level is chosen', controls), ('Behavior', behavior), ('Research', runs)):
        print(f'### {title}\n')
        print('\n'.join(table))
        print()

if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
