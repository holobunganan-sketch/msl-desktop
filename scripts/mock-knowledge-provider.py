"""Loopback-only synthetic provider. Never logs request bodies or credentials."""
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
import json,time
METRICS={'requests':{},'scope_isolation':True,'history_received':False,'kol_categories':False,'document_received':False}
def respond(payload):
    model=payload['model'];METRICS['requests'][model]=METRICS['requests'].get(model,0)+1
    data=json.loads(next(m['content'] for m in payload['messages'] if m['role']=='user'))
    pack=data['evidence'];sources=pack['sources'];system=next((m['content'] for m in payload['messages'] if m['role']=='system'),'')
    time.sleep(2)
    if model=='qa-forged':return json.dumps({'claims':[{'text':'Invented','basis':'fact','citations':[{'source_id':'task:9999999','quote':'fabricated'}]}],'gaps':[]})
    if model=='qa-repair' and 'previous response failed' not in system:return 'not valid json'
    if model.startswith('qa'):
        METRICS['history_received'] |= bool(data.get('history_context_only'))
        if pack['scope_ids'] and any(s['kind']=='task' and json.loads(s['text']).get('work_id') not in pack['scope_ids'] for s in sources):METRICS['scope_isolation']=False
        METRICS['document_received'] |= any(s['kind']=='document' for s in sources)
        source=next((s for s in sources if s['kind']=='task'),sources[0]);row=json.loads(source['text'])
        quote=row.get('title','范围统计 / Scope counts')
        return json.dumps({'claims':[{'text':'当前记录中有一项需要继续推进：'+quote,'basis':'fact','citations':[{'source_id':source['id'],'quote':quote}]},{'text':'建议先核对这项工作的最新反馈，再决定下一步。','basis':'inference','citations':[{'source_id':source['id'],'quote':quote}]}],'gaps':['现有资料不能确认事项已经完成。']},ensure_ascii=False)
    note_sources=[s for s in sources if s['kind']=='kol_note']
    citations=[{'source_id':s['id'],'quote':json.loads(s['text'])['content'][:120]} for s in note_sources[:3]]
    METRICS['kol_categories']=True
    works=[s for s in sources if s['kind']=='work'];work_id=works[0]['entity_id'] if works else None
    actions=[] if data['purpose'] in ('prepare','synthesize') else [dict(enabled=True,kind='task',title='合成：补充专家所需的随访证据',work_id=work_id,notes='核对后再准备材料',waiting_for='',at=int(time.time())+86400,time_basis='inferred',time_reason='下次交流前预留准备时间，待用户确认',citations=citations),dict(enabled=True,kind='waiting',title='合成：等待进一步反馈',work_id=work_id,notes='',waiting_for='专家反馈',at=None,time_basis='unknown',time_reason='',citations=citations)]
    return json.dumps({'summary':'交流中出现随访证据需求；需要了解具体障碍和合作问题。','citations':citations,'insights':[dict(title='随访证据与实践需求仍需进一步澄清',categories=['practice_barrier','evidence_need','research_opportunity'],observation=citations[0]['quote'],implication='可能需要更贴近临床情境的证据材料。',uncertainty='尚未确认适用患者群和研究条件。',next_question='哪些结局最影响您的临床判断？',citations=citations)],'actions':actions},ensure_ascii=False)
class Handler(BaseHTTPRequestHandler):
    def log_message(self,*args):pass
    def do_GET(self):
        value=METRICS if self.path=='/metrics' else {'ready':True}
        self.send_response(200);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(json.dumps(value).encode())
    def do_POST(self):
        try:
            payload=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            content=respond(payload)
            body=json.dumps({'choices':[{'message':{'role':'assistant','content':content},'finish_reason':'stop'}]}).encode()
            self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(body)));self.end_headers();self.wfile.write(body)
        except (BrokenPipeError,ConnectionResetError):pass
        except Exception:
            self.send_error(500,'Synthetic fixture failed')
if __name__=='__main__':ThreadingHTTPServer(('127.0.0.1',9481),Handler).serve_forever()
