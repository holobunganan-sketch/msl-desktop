type Greeting = {title:string;note:string};
type PreferenceStorage = Pick<Storage,'getItem'|'setItem'>;
const greetings:Array<{zh:Greeting;en:Greeting}> = [
  {zh:{title:'专注于有价值的医学连接。',note:'让每一次专业对话，都有持续的跟进。'},en:{title:'Make room for meaningful conversations.',note:'Let each professional conversation lead to a thoughtful next step.'}},
  {zh:{title:'从上次的进展，接着往前。',note:'留住交流中的线索，准备下一次沟通。'},en:{title:'Pick up where your work left off.',note:'Keep the threads of each conversation ready for the next one.'}},
  {zh:{title:'把时间留给值得跟进的事。',note:'整理交给工作台，判断留给您。'},en:{title:'Make time for what deserves follow-up.',note:'Let the workspace organize. Keep the decisions yours.'}},
  {zh:{title:'认真倾听，让线索慢慢清晰。',note:'专家的一句话，也值得好好记住。'},en:{title:'Listen closely. Let the picture become clearer.',note:'A few words from an expert can be worth remembering.'}},
  {zh:{title:'带着清晰的问题，开始今天。',note:'保留事实与疑问，为下一步做好准备。'},en:{title:'Start today with a clear question.',note:'Keep facts and open questions in view as you prepare the next step.'}},
  {zh:{title:'让每一次交流，都有下文。',note:'从理解需求开始，把跟进落到具体事情上。'},en:{title:'Give every conversation a next chapter.',note:'Understand the need, then make the follow-up concrete.'}},
  {zh:{title:'循着线索，稳稳推进。',note:'连接项目、证据与交流，一次推进一小步。'},en:{title:'Follow the threads. Make steady progress.',note:'Connect projects, evidence and conversations, one step at a time.'}},
  {zh:{title:'留一点空间，给专业判断。',note:'让工作自然流动，让重要的事保持清楚。'},en:{title:'Leave room for professional judgment.',note:'Keep your work moving and the important things clear.'}}
];

// One choice per application session, excluding the previous opening when possible.
export function createSessionGreeting(storage?:PreferenceStorage, random= Math.random) {
  const key='msl.dashboard.previousGreeting';
  let previous=-1;
  try { const saved=storage?.getItem(key); if(saved!=null&&/^\d+$/.test(saved))previous=Number(saved); } catch { /* Optional local preference. */ }
  const choices=greetings.map((_,index)=>index).filter(index=>index!==previous);
  const sample=random();
  const position=Math.min(choices.length-1,Math.max(0,Math.floor((Number.isFinite(sample)?sample:0)*choices.length)));
  const selected=choices[position];
  try { storage?.setItem(key,String(selected)); } catch { /* Greeting still works without storage. */ }
  return (locale:string):Greeting => greetings[selected][locale==='en-US'?'en':'zh'];
}

let sessionGreeting:ReturnType<typeof createSessionGreeting>|undefined;
export function dashboardGreeting(locale:string):Greeting {
  if(typeof window==='undefined')return greetings[0][locale==='en-US'?'en':'zh'];
  if(!sessionGreeting){
    let storage:PreferenceStorage|undefined;
    try {storage=window.localStorage;} catch { /* Storage access may be disabled. */ }
    sessionGreeting=createSessionGreeting(storage);
  }
  return sessionGreeting(locale);
}

export function fileChangeLabel(event:string,locale:string):string {
  const labels:Record<string,[string,string]>={
    'file.created':['新资料','Added'], 'file.modified':['已更新','Updated'],
    'file.deleted':['已移除','Removed'], 'file.renamed':['已重命名','Renamed'],
    'file.moved':['已移动','Moved'], '更新':['已更新','Updated']
  };
  return (labels[event]??['资料变化','Material changed'])[locale==='en-US'?1:0];
}
