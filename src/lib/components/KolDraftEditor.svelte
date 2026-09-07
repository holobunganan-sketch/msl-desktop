<script lang="ts">
 import {locale} from '$lib/i18n';
 import {untrack} from 'svelte';
 import {command,normalizeError} from '$lib/services/api';
 import {json,categories,localTime,parseLocalTime,type KolDraft,type KolOutput,type Project,type Pack} from '$lib/services/knowledge';
 import KnowledgeSources from './KnowledgeSources.svelte';
 let {draft,projects,onchange}:{draft:KolDraft;projects:Project[];onchange:()=>void}=$props();
 let en=$derived($locale==='en-US');
 let value=$state<KolOutput>(json(untrack(()=>draft.payload_json),{summary:'',insights:[],actions:[],citations:[]}));
 let error=$state(''),busy=$state(false),confirming=$state(false);
 let pack=$derived(json<Pack|null>(draft.evidence_json,null));
 let readonly=$derived(draft.status!=='pending');
 async function decide(decision:string){busy=true;error='';try{await command('review_kol_draft',{id:draft.id,revision:draft.revision,decision,payload:value});confirming=false;onchange();}catch(e){error=normalizeError(e);}finally{busy=false;}}
 function time(index:number,text:string){try{value.actions[index].at=parseLocalTime(text);value.actions[index].time_basis=text?'explicit':'unknown';value.actions[index].time_reason=text?(en?'Time selected by user':'用户指定时间'):'';}catch(e){error=normalizeError(e);}}
</script>
<section class="k-card k-stack" data-testid="kol-draft-editor">
 <div class="k-head"><h2>{draft.purpose==='prepare'?(en?'Prepare for your next conversation':'下次交流准备'):draft.purpose==='synthesize'?(en?'Across-expert insights':'跨专家洞察'):(en?'Review this interaction':'一起整理这次交流')}</h2><span class="k-pill">{draft.status==='pending'?(en?'Draft · your decision':'草稿 · 等您决定'):draft.status==='confirmed'?(en?'Confirmed':'已确认'):(en?'Rejected':'已拒绝')}</span></div>
 <p class="muted">{en?'Original notes stay unchanged. Edit the interpretation; select the follow-ups you want.':'原始交流内容保持不变。您可以修改理解、归类和后续动作，确认选中的事项。'}</p>
 <label>{en?'Overview':'交流脉络 / 准备摘要'}<textarea data-testid="kol-draft-summary" bind:value={value.summary} disabled={readonly} rows="3"></textarea></label>
 <KnowledgeSources citations={value.citations} {pack}/>
 {#if pack}<p class="muted">{en?'Available original interactions':'范围内原始交流'}：{pack.counts.kol_note??0} · {en?'Experts':'专家'}：{pack.counts.expert??0} · {en?'Omitted records':'本轮未纳入'}：{pack.omitted}。{en?'Repeated records from one expert are not independent consensus.':'同一专家的多次表达不代表独立共识。'}</p>{/if}
 {#each value.insights as insight,i(i)}
 <section class="insight k-stack"><div class="k-head"><h3>{en?'Insight':'洞察'} {i+1}</h3>{#if !readonly}<button onclick={()=>value.insights=value.insights.filter((_,index)=>index!==i)}>{en?'Leave out':'本次不保留'}</button>{/if}</div>
 <label>{en?'What should we pay attention to?':'值得关注的是什么？'}<input bind:value={insight.title} disabled={readonly}/></label>
 <div class="k-actions">{#each categories as category}<label class="k-check"><input type="checkbox" checked={insight.categories.includes(category[0])} disabled={readonly} onchange={e=>insight.categories=e.currentTarget.checked?[...insight.categories,category[0]]:insight.categories.filter(c=>c!==category[0])}/>{category[en?2:1]}</label>{/each}</div>
 <div class="k-fields"><label>{en?'Observed or said':'观察到的事实 / 专家表达'}<textarea bind:value={insight.observation} disabled={readonly}></textarea></label><label>{en?'Possible significance':'可能意味着什么'}<textarea bind:value={insight.implication} disabled={readonly}></textarea></label><label>{en?'What remains uncertain?':'还有哪些不确定性？'}<textarea bind:value={insight.uncertainty} disabled={readonly}></textarea></label><label>{en?'Next useful question':'下次值得追问的问题'}<textarea bind:value={insight.next_question} disabled={readonly}></textarea></label></div>
 <KnowledgeSources citations={insight.citations} {pack}/>
 </section>{/each}
 {#if value.actions.length}<div class="k-rule"><h3>{en?'Follow-ups to bring into your workbench':'带入工作台的后续事项'}</h3><p class="muted">{en?'Only checked actions are created. Inferred times are proposals until you confirm.':'仅写入勾选的事项；AI 推算的时间需要您核对后确认。'}</p></div>{/if}
 {#each value.actions as action,i(i)}<section class="insight k-stack" class:dimmed={!action.enabled}>
 <label class="k-check"><input data-testid={`kol-action-enabled-${i}`} type="checkbox" bind:checked={action.enabled} disabled={readonly}/><strong>{en?'Create this follow-up':'加入这条跟进'}</strong></label>
 <div class="k-fields"><label>{en?'Action':'要做什么'}<input data-testid={`kol-action-title-${i}`} bind:value={action.title} disabled={readonly}/></label><label>{en?'Destination':'去向'}<select bind:value={action.kind} disabled={readonly}><option value="task">{en?'Task':'任务计划'}</option><option value="waiting">{en?'Waiting':'等待事项'}</option><option value="calendar">{en?'Calendar':'日历'}</option><option value="inbox">{en?'Inbox':'收件箱'}</option></select></label><label>{en?'Project':'归属项目'}<select bind:value={action.work_id} disabled={readonly}><option value={null}>{en?'Independent matter':'临时事务'}</option>{#each projects as p(p.id)}<option value={p.id}>{p.title}</option>{/each}</select></label><label>{en?'When (optional for tasks and waiting)':'时间（任务与等待可暂不安排）'}<input type="datetime-local" value={localTime(action.at)} disabled={readonly} onchange={e=>time(i,e.currentTarget.value)}/></label>
 {#if action.kind==='waiting'}<label>{en?'Waiting for whom or what?':'在等谁 / 等什么'}<input bind:value={action.waiting_for} disabled={readonly}/></label>{/if}<label>{en?'Details':'补充说明'}<textarea bind:value={action.notes} disabled={readonly}></textarea></label></div>
 {#if action.at}<p class="muted">{action.time_basis==='inferred'?(en?'AI-proposed time — please verify':'AI 建议时间，请核对'):(en?'Specified time':'指定时间')} · {action.time_reason}</p>{/if}
 <KnowledgeSources citations={action.citations} {pack}/></section>{/each}
 {#if error}<div class="k-error" role="alert">{error}</div>{/if}
 {#if !readonly}<footer class="k-actions">{#if confirming}<span>{en?`Keep ${value.insights.length} insights and create ${value.actions.filter(a=>a.enabled).length} follow-ups?`:`保留 ${value.insights.length} 条洞察，并创建 ${value.actions.filter(a=>a.enabled).length} 条跟进事项？`}</span><button class="primary" data-testid="kol-commit" disabled={busy} onclick={()=>decide('confirm')}>{en?'Confirm and save':'确认写入'}</button><button onclick={()=>confirming=false}>{en?'Back':'返回修改'}</button>{:else}<button class="primary" data-testid="kol-confirm" disabled={busy} onclick={()=>confirming=true}>{en?'Confirm selected results':'确认选中的结果'}</button><button disabled={busy} onclick={()=>decide('save')}>{en?'Save edits for later':'保存修改，稍后决定'}</button><button disabled={busy} onclick={()=>decide('reject')}>{en?'Reject this draft':'拒绝本次草稿'}</button>{/if}</footer>{/if}
</section>
<style>.insight{padding:20px;border:1px solid var(--color-border);border-radius:16px;background:var(--color-bg)}.dimmed{opacity:.72}</style>
