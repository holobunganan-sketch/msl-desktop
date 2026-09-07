<script lang="ts">
  import {locale,t} from "$lib/i18n";
  let {works,value=$bindable("all")}:{works:Array<{id:number;title:string}>;value?:string}=$props();
</script>
<label class="project-filter"><span>{t("scope.label",{},$locale)}</span>
  <select data-testid="project-filter" bind:value>
    <option value="all">{t("scope.all",{},$locale)}</option>
    <option value="independent">{t("scope.independent",{},$locale)}</option>
    {#each works as work(work.id)}<option value={String(work.id)}>{work.title}</option>{/each}
  </select>
</label>
<style>
  .project-filter{display:flex;align-items:center;flex-wrap:wrap;gap:10px;margin:14px 0 20px;font-size:14px;color:var(--color-muted)}
  select{max-width:100%;min-width:min(250px,100%);min-height:42px;padding:8px 12px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);color:var(--color-text);font:inherit;font-size:15px}
</style>
