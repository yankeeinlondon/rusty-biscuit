"""Lint for research contracts: every property has a description that carries meaning.

Usage: contract_description_lint.py <contract file>...

A prototype. It reports a property with no description, a description under
six words, one that restates the property name, wording repeated across
properties, an object written as a string, and a description that contains a
colon followed by a space.
"""
import yaml,sys,re,collections
def walk(name,node,path,out):
    if isinstance(node,dict):
        for k,v in node.items(): walk(k,v,path+[k],out)
    elif isinstance(node,list):
        for i,v in enumerate(node): walk(name,v,path+[f'arm{i}'],out)
    elif isinstance(node,str):
        t,_,d=node.partition(' -> ')
        out.append(('/'.join(path),name,t.strip(),d.strip()))
props=[]
early=[]
for f in sys.argv[1:]:
    for n,line in enumerate(open(f),1):
        if line.lstrip().startswith('#') or ' -> ' not in line: continue
        if ': ' in line.split(' -> ',1)[1]:
            early.append((f'{f}:{n}','the description contains a colon followed by a space, which YAML reads as a mapping'))
if early:
    print(f"{len(early)} problems")
    for p in early: print(' -',p[0],'::',p[1])
    sys.exit(1)
for f in sys.argv[1:]:
    d=yaml.safe_load(open(f))
    walk('',d.get('$schema') or d.get('types'),[f],props)
problems=[]; seen=collections.defaultdict(list)
for path,name,t,d in props:
    words=re.findall(r"[A-Za-z']+",d)
    if not d: problems.append((path,'no description'))
    elif len(words)<6: problems.append((path,f'too short ({len(words)} words): {d}'))
    else:
        namewords=set(re.split(r'[_-]',name.lower()))
        content=[w.lower() for w in words if w.lower() not in {'the','a','an','of','for','this','that','to','is','in','on'}]
        if set(content)<=namewords|{'id','ids'}: problems.append((path,f'restates the name: {d}'))
    if d: seen[re.sub(r'\b(mechanism|channel|profile|case|method|entry|finding|guarantee|observation|test|states|decision)\b','X',d.lower())].append(path)
    if '{' in t or t.strip().startswith('"'): problems.append((path,'string object notation'))
for k,v in seen.items():
    if len(v)>1: problems.append((v[0],f'same wording on {len(v)} properties: "{k}"'))
print(f"{len(props)} properties checked, {len(problems)} problems")
for p in problems: print(' -',p[0].split('/',1)[1] if '/' in p[0] else p[0],'::',p[1])
sys.exit(1 if problems else 0)
