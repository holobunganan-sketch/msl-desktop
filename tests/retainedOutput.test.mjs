import test,{after} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
const server=await createServer({server:{middlewareMode:true,hmr:false},logLevel:'error'});
after(()=>server.close());
const {readableOutput}=await server.ssrLoadModule('/src/lib/services/retainedOutput.ts');
test('unstructured answers and malformed JSON remain fully readable',()=>{
  for(const raw of ['请核对专家的归属。','{"summary":"保留未闭合的文字','<script>window.hacked=true</script>']){
    assert.equal(readableOutput(raw)[0].text,raw);
  }
});
test('structured output separates each item and keeps unknown content',()=>{
  const result=readableOutput(JSON.stringify({summary:'两项安排',proposals:[{title:'需要核对',payload:{notes:'完整观点',unexpected_field:'开放主题'}},{title:'正常建议',reason:'原话依据'}],other:'额外发现'}));
  assert.equal(result.length,4);
  assert.match(result[1].text,/开放主题/);
  assert.match(result[2].text,/原话依据/);
  assert.match(result[3].text,/额外发现/);
});
