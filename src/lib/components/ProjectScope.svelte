<script lang="ts">
  import { locale, t, translateStatus } from "$lib/i18n";
  let { works, value = $bindable<number | null>(null), required = false, disabled = false, id = "project-scope", onchange }: {
    works: Array<{id:number;title:string;status:string}>; value?: number | null;
    required?: boolean; disabled?: boolean; id?: string; onchange?: (value:number|null)=>void;
  } = $props();
</script>

<label class="project-scope" for={id}>
  <span>{t("scope.label",{},$locale)}{required ? " *" : ""}</span>
  <select {id} data-testid={id} {disabled} {required} value={value??""} onchange={event=>{value=event.currentTarget.value===""?null:Number(event.currentTarget.value);onchange?.(value);}}>
    <option value="" disabled={required}>{t(required?"scope.choose":"scope.independent",{},$locale)}</option>
    {#each works.filter(work=>work.status!=="archived"||work.id===value) as work(work.id)}
      <option value={work.id} disabled={work.status==="archived"}>{work.title}{work.status==="archived"?` · ${translateStatus(work.status,$locale)}`:""}</option>
    {/each}
    {#if value!==null && !works.some(work=>work.id===value)}<option value={value} disabled>{t("scope.missing",{},$locale)}</option>{/if}
  </select>
</label>

<style>
  .project-scope{display:grid;gap:7px;min-width:0;align-content:start;font-size:14px;color:var(--color-muted)}
  select{width:100%;min-width:0;min-height:42px;padding:9px 11px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);color:var(--color-text);font:inherit;font-size:15px}
  select:disabled{opacity:.7}
</style>
