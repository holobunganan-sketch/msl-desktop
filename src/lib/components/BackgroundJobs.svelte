<script lang="ts">
  import { onMount } from "svelte";
  import { aiJobs, refreshJobs, type AiJob } from "$lib/stores/aiJobs";
  import { locale } from "$lib/i18n";
  import Icon from "$lib/components/ui/Icon.svelte";
  let expanded=$state(false);
  let error=$state("");
  let running=$derived($aiJobs.filter(job=>job.status==="running"));
  const label=(command:string)=>({ask_workbench:["工作台问答","Workbench Q&A"],analyze_kol:["专家与洞察","Experts & insights"],refresh_project_cognition:["更新项目认知","Refresh cognition"],organize_inbox_item:["整理一句记录","Organize observation"],run_analysis_now:["秘书分析","Secretary analysis"],start_workspace_work_draft:["项目整理","Project organization"],generate_brief:["秘书简报","Daily brief"],translate_text:["AI 翻译","Translation"],generate_report:["工作报告","Work report"],retry_report:["重试报告","Retry report"],retry_analysis_run:["重试分析","Retry analysis"]}[command]?.[$locale==="en-US"?1:0]??command);
  const status=(value:string)=>({running:["后台进行中","Running"],completed:["已完成","Completed"],failed:["未完成","Failed"],interrupted:["已中断","Interrupted"]}[value]?.[$locale==="en-US"?1:0]??value);
  function visit(job:AiJob){expanded=false;if(job.command==="ask_workbench"||job.command==="analyze_kol"){const result=job.result as {session_id?:number;expert_id?:number}|null;window.dispatchEvent(new CustomEvent("dashboard:navigate",{detail:job.command==="ask_workbench"?{view:"qa",id:result?.session_id}:{view:"kol",id:result?.expert_id??job.args.expertId}}));return;}window.dispatchEvent(new CustomEvent("dashboard:navigate",{detail:job.command==="refresh_project_cognition"?(job.args.scope==="work"?"works":"workspace"):job.command==="translate_text"?"translation":job.command.includes("report")?"reports":job.command==="generate_brief"?"today":"review"}));}
  onMount(()=>{
    const refresh=()=>void refreshJobs().then(()=>error="").catch(()=>error=$locale==="en-US"?"Unable to refresh task status":"暂时无法刷新任务状态");
    const dismissOutside=(event:MouseEvent)=>{if(event.target instanceof Element&&!event.target.closest('.job-center'))expanded=false;};
    const dismissEscape=(event:KeyboardEvent)=>{if(event.key==='Escape')expanded=false;};
    refresh();const timer=setInterval(refresh,1800);
    document.addEventListener('click',dismissOutside);
    document.addEventListener('keydown',dismissEscape);
    return()=>{clearInterval(timer);document.removeEventListener('click',dismissOutside);document.removeEventListener('keydown',dismissEscape);};
  });
</script>

<div class="job-center">
  <button class="job-toggle" data-testid="background-jobs-toggle" onclick={()=>expanded=!expanded} aria-expanded={expanded}>
    <Icon name={running.length?"activity":"sparkles"} size={18}/><span><strong>{$locale==="en-US"?"Background tasks":"后台任务"}</strong><small aria-live="polite">{running.length?`${running.length} · ${status("running")}`:$locale==="en-US"?"View recent results":"查看最近结果"}</small></span>
  </button>
  {#if expanded}
    <section class="job-popover" data-testid="background-jobs-panel" aria-label={$locale==="en-US"?"Background tasks":"后台任务"}>
      <header><strong>{$locale==="en-US"?"Tasks continue as you work":"切换页面，任务继续"}</strong><button onclick={()=>expanded=false} aria-label={$locale==="en-US"?"Close":"关闭"}>×</button></header>
      <p>{$locale==="en-US"?"Results are saved locally. Quit interrupts unfinished tasks; they can be started again.":"结果保存在本机。退出软件会中断未完成任务，可重新发起。"}</p>
      {#if error}<p role="alert">{error}</p>{/if}
      <div class="job-list">
        {#each $aiJobs.slice(0,12) as job(job.id)}
          <button class="job" onclick={()=>visit(job)}><span><strong>{label(job.command)}</strong><small>#{job.id} · {new Date(job.created_at*1000).toLocaleTimeString($locale,{hour:"2-digit",minute:"2-digit"})}</small></span><span class:failed={job.status==="failed"||job.status==="interrupted"}>{status(job.status)}</span>{#if job.error}<small class="job-error">{job.error}</small>{/if}</button>
        {:else}<p>{$locale==="en-US"?"No recent tasks":"暂无后台任务"}</p>{/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .job-center{flex-shrink:0;position:relative;margin-top:auto;min-width:0}.job-toggle{display:flex;align-items:center;gap:10px;width:100%;padding:12px;border:1px solid var(--color-border);border-radius:14px;background:var(--color-surface);color:var(--color-text);text-align:left;cursor:pointer}.job-toggle span{display:grid;gap:4px;min-width:0}.job-toggle strong{font-size:14px}.job-toggle small{font-size:12px;color:var(--color-muted)}.job-popover{position:fixed;z-index:120;left:calc(var(--sidebar-width) + 12px);bottom:20px;width:min(440px,calc(var(--viewport-width) - var(--sidebar-width) - 32px));max-height:calc(var(--viewport-height) - 40px);padding:20px;background:var(--color-surface);border:1px solid var(--color-border);border-radius:18px;box-shadow:0 12px 50px #23333d26;display:flex;flex-direction:column;gap:12px}.job-popover header{display:flex;align-items:center;justify-content:space-between;gap:14px}.job-popover header button{background:transparent;border:0;font-size:24px;color:var(--color-muted);cursor:pointer}.job-popover p{margin:0;color:var(--color-muted);font-size:14px;line-height:1.6}.job-list{overflow:auto;min-height:0}.job{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:10px;padding:14px 4px;width:100%;text-align:left;background:transparent;border:0;border-top:1px solid var(--color-border);color:var(--color-text);cursor:pointer}.job>span:first-child{display:grid;gap:4px}.job small{color:var(--color-muted);font-size:13px}.job>span{font-size:14px}.job .failed{color:var(--color-danger)}.job-error{grid-column:1/-1;overflow-wrap:anywhere;line-height:1.5}
  @media(max-width:760px){.job-toggle{justify-content:center;padding:12px 4px}.job-toggle>span{display:none}.job-popover{left:calc(var(--sidebar-collapsed-width) + 12px);width:min(440px,calc(var(--viewport-width) - var(--sidebar-collapsed-width) - 32px))}}
</style>
