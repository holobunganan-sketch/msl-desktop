type Section={title:string;text:string};
const labels:Record<string,string>={summary:'分析摘要',proposals:'建议',title:'标题',reason:'整理依据',notes:'补充说明',goal:'目标',current_state:'当前进展',next_step:'下一步',observation:'观察',implication:'可能影响',uncertainty:'待核实',next_question:'下一次交流',description:'说明',content:'内容',text:'内容',payload:'建议填写内容',work_id:'项目编号',expert_id:'专家编号',source_refs:'模型提供的来源（待核对）',field_evidence:'字段依据（待核对）',kind:'类型',operation:'动作',confidence:'模型自报置信度'};
function describe(value:unknown,depth=0):string{
  if(value===null||value===undefined)return '';
  if(typeof value!=='object')return String(value);
  if(depth>12)return JSON.stringify(value);
  if(Array.isArray(value))return value.map(v=>describe(v,depth+1)).filter(Boolean).map(v=>`• ${v}`).join('\n');
  return Object.entries(value).map(([key,v])=>`${labels[key]??key}：${describe(v,depth+1)}`).join('\n');
}
export function readableOutput(raw:string):Section[]{
  if(!raw)return [];
  const clean=raw.trim().replace(/^```(?:json)?\s*\n?/i,'').replace(/\n?```$/,'');
  try{
    const value=JSON.parse(clean);
    if(!value||typeof value!=='object')return [{title:'',text:raw}];
    if(!Array.isArray(value.proposals))return [{title:'',text:describe(value)}];
    const result:Section[]=[];
    if(value.summary)result.push({title:'分析摘要',text:describe(value.summary)});
    value.proposals.forEach((item:unknown,index:number)=>{const proposal=item&&typeof item==='object'?item as Record<string,unknown>:{};result.push({title:`${index+1}. ${String(proposal.title??'模型建议')}`,text:describe(item)});});
    const rest=Object.fromEntries(Object.entries(value).filter(([key])=>!['summary','proposals'].includes(key)));
    if(Object.keys(rest).length)result.push({title:'其他内容',text:describe(rest)});
    return result.length?result:[{title:'',text:raw}];
  }catch{return [{title:'',text:raw}];}
}
