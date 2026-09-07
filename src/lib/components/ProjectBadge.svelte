<script lang="ts">
  import {locale,t} from "$lib/i18n";
  let {works,workId}:{works:Array<{id:number;title:string}>;workId:number|null}=$props();
</script>
<span class="scope-badge" class:independent={workId===null} data-testid="project-badge">{workId===null?t("scope.independent",{},$locale):`${t("work.title",{},$locale)} · ${works.find(work=>work.id===workId)?.title??t("scope.missing",{},$locale)}`}</span>
<style>
  .scope-badge{display:inline-block;max-width:100%;padding:3px 9px;border-radius:7px;background:var(--color-primary-soft);color:var(--color-primary);font-size:14px;line-height:1.6;overflow-wrap:anywhere}
  .independent{background:var(--color-surface-muted);color:var(--color-muted)}
</style>
