"""Synthetic-only cognition persistence after an actual application restart."""
import hashlib
import json
import os
from pathlib import Path
import runpy

assert all('.test-runtime' in Path(os.environ[k]).parts for k in ('APPDATA','LOCALAPPDATA','TEMP','TMP'))
Page=runpy.run_path(str(Path(__file__).with_name('release-functional-cdp.py')))['Page']
page=Page()
state=json.loads((Path(os.environ['MSL_ARTIFACTS'])/'cognition-state.json').read_text(encoding='utf-8'))
root=Path(state['root']).resolve()
assert '.test-runtime' in root.parts
def invoke(command,**args):
    return page.eval(f'window.__TAURI_INTERNALS__.invoke({json.dumps(command)},{json.dumps(args)})')
entry=invoke('get_project_cognition',scope='work',scopeId=state['work_id'])
detail=invoke('get_work_detail',id=state['work_id'])
hashes={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob('*') if p.is_file()}
checks=[Path(entry['path']).is_file(),entry['document_count']==3,'认知合成' in entry['markdown'],
        hashes==state['source_hashes'],any(t['status']=='next' for t in detail['tasks']),
        any(w['status']=='open' for w in detail['waiting']),
        any(r['undone_at'] is not None for r in invoke('list_confirmation_receipts'))]
print(f'COGNITION_PERSISTENCE_PASS={sum(checks)} FAIL={len(checks)-sum(checks)}')
page.ws.close()
raise SystemExit(0 if all(checks) else 1)
