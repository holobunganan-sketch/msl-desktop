<script lang="ts">
  import { locale, t } from "$lib/i18n";
  import ProjectScope from "$lib/components/ProjectScope.svelte";
  let { kind, workId, operation, works, disabled = false, onkindchange, onworkchange }: {
    kind:string;workId:number|null;operation:string;works:Array<{id:number;title:string;status:string}>;
    disabled?:boolean;onkindchange:(kind:string)=>void;onworkchange:(id:number|null)=>void;
  }=$props();
  let projectMode=$state(false);
  const scopeId=$props.id();
  const destination=$derived(kind==="resume_point"||projectMode?"project":kind);
  const tx=(key:Parameters<typeof t>[0])=>t(key,{},$locale);
  function change(value:string){
    projectMode=value==="project";
    onkindchange(projectMode?"resume_point":value);
    if(value==="work"||value==="inbox")onworkchange(null);
  }
</script>

<div class="proposal-routing" data-testid="proposal-routing">
  <label><span>{tx("dashboard.destination")}</span>
    <select data-testid="proposal-destination" value={destination} onchange={event=>change(event.currentTarget.value)} {disabled}>
      <option value="work">{tx(kind==="work"&&operation==="update"?"scope.updateProject":"scope.newProject")}</option>
      <option value="project">{tx("scope.intoProject")}</option>
      <option value="task">{tx("proposal.kind.task")}</option>
      <option value="waiting">{tx("proposal.kind.waiting")}</option>
      <option value="calendar">{tx("proposal.kind.calendar")}</option>
      <option value="inbox">{tx("scope.keepInbox")}</option>
    </select>
  </label>
  {#if destination==="project"}
    <label><span>{tx("scope.itemType")}</span><select data-testid="proposal-project-kind" value={kind} onchange={event=>onkindchange(event.currentTarget.value)} disabled={disabled||workId===null}>
      <option value="resume_point">{tx("scope.progress")}</option><option value="task">{tx("proposal.kind.task")}</option><option value="waiting">{tx("proposal.kind.waiting")}</option><option value="calendar">{tx("proposal.kind.calendar")}</option>
    </select></label>
  {/if}
  {#if ["task","waiting","calendar","resume_point"].includes(kind)}
    <ProjectScope {works} value={workId} required={destination==="project"} {disabled} id={scopeId} onchange={onworkchange}/>
  {/if}
  <p>{tx(kind==="work"?"scope.newHint":kind==="inbox"?"scope.inboxHint":"scope.itemHint")}</p>
</div>

<style>
  .proposal-routing{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,220px),1fr));gap:12px;min-width:0;grid-column:1/-1}
  label{min-width:0;display:grid;align-content:start;gap:7px;font-size:14px;color:var(--color-muted)}
  select{width:100%;min-width:0;min-height:42px;border:1px solid var(--color-border);border-radius:10px;padding:9px 11px;background:var(--color-surface);color:var(--color-text);font:inherit;font-size:15px}
  p{grid-column:1/-1;margin:0;font-size:13px;line-height:1.6;color:var(--color-muted)}
</style>
