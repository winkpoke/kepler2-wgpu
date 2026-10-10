import json,gzip,sys,re
from pathlib import Path

def load(p):
    b=Path(p).read_bytes()
    return json.loads(gzip.decompress(b) if str(p).endswith('.gz') else b)

def surface(d):
    idx=d['index']; paths=d['paths']; root=str(d['root']); seen=set(); result={}
    def get(id): return idx.get(str(id))
    def norm(x):
        if isinstance(x,dict):
            out={}
            for k,v in x.items():
                if k in ['impls','implementations','items','fields','variants','discriminant']:continue
                if k=='id':
                    value=paths.get(str(v),{}).get('path', get(v).get('name') if get(v) else 'UNRESOLVED')
                    if isinstance(value,list) and value[0] in ['wgpu_types','winit']:value=[value[0],value[-1]]
                    out[k]=value
                elif k=='tuple' and isinstance(v,list) and all(n is None or isinstance(n,int) for n in v):out[k]=['<field>']*len(v)
                else:out[k]=norm(v)
            return out
        if isinstance(x,list): return [norm(v) for v in x]
        return x
    def walk(id,p,force=False):
        it=get(id)
        if not it or it['crate_id']!=0 or (not force and it['visibility']!='public'): return
        key=(str(id),tuple(p))
        if key in seen:return
        seen.add(key)
        kind,data=next(iter(it['inner'].items()))
        if kind=='use':
            if data['id'] is not None:
                target=get(data['id'])
                if target:
                    targetp=p[:-1] if data['is_glob'] else p[:-1]+[data['name']]
                    walk(data['id'],targetp,True)
            return
        name='::'.join(p)
        result[name]={'kind':kind,'declaration':norm(data)}
        if kind=='module':
            for child in data['items']:
                item=get(child)
                if item:walk(child,p+[item['name'] or '<use>'])
        if kind in ['struct','union','enum','trait','variant']:
            fields=data.get('fields',[])
            if kind=='struct':
                shape=data['kind']
                if isinstance(shape,dict):fields=next(iter(shape.values())).get('fields',[]) if isinstance(next(iter(shape.values())),dict) else next(iter(shape.values()))
            elif kind=='variant':
                shape=data['kind']
                if isinstance(shape,dict):
                    value=next(iter(shape.values()));fields=value.get('fields',[]) if isinstance(value,dict) else value
            for child in fields or []:
                item=get(child)
                if item:walk(child,p+[item['name'] or '<field>'],True)
            for child in data.get('variants',[]) or data.get('items',[]):
                item=get(child)
                if item:walk(child,p+[item['name'] or '<item>'],True)
            for implid in data.get('impls',[]):
                imp=get(implid)
                if not imp:continue
                impl=imp['inner'].get('impl',{})
                if impl.get('trait') is None:
                    for child in impl.get('items',[]):
                        item=get(child)
                        if item:walk(child,p+[item['name'] or '<method>'])
    walk(root,[idx[root]['name']],True)
    return result

base=surface(load(sys.argv[1]));candidate=surface(load(sys.argv[2]))
out={'baseline_count':len(base),'candidate_count':len(candidate),'removed':sorted(base.keys()-candidate.keys()),'added':sorted(candidate.keys()-base.keys()),'changed':[{ 'path':k,'baseline':base[k],'candidate':candidate[k]} for k in sorted(base.keys()&candidate.keys()) if base[k]!=candidate[k]]}
Path(sys.argv[3]).write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
print('Counts',len(base),len(candidate),'Removed',len(out['removed']),'Added',len(out['added']),'Changed',len(out['changed']))
for x in out['removed']:print('removed',x)
for x in out['changed']:print('changed',x['path'])

# Classify only the explicitly reviewed existing-item changes. Re-export aliases
# are counted separately. Upstream crate version identity requires the consumer
# fixture and dependency-tree evidence in addition to this declaration audit.
def decision(path):
    if re.search(r"::(?:App|Renderable)::render$", path) or path.endswith("::KeplerError::Surface::0"):
        return "P2"
    if re.search(r"::PassContext(?:::(?:descriptor|new|pass))?$", path) or path.endswith("::PassExecutor::execute_frame"):
        return "P3"
    if path.endswith("::App::resize"):
        return "P1 upstream PhysicalSize identity"
    return None
for change in out['changed']:
    change['decision'] = decision(change['path'])
Path(sys.argv[3]).write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
if out['removed'] or any(c['decision'] is None for c in out['changed']):
    raise SystemExit('Unreviewed removal or existing-item change')
