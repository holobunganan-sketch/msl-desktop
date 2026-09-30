<script lang="ts">
  import { locale } from '$lib/i18n';
  import { analysisCoverage } from '$lib/services/analysisCoverage';
  let { counts }: { counts: string | null | undefined } = $props();
  const en = $derived($locale === 'en-US');
  const coverage = $derived(analysisCoverage(counts, $locale));
</script>
{#if coverage}
  <details class="coverage" data-testid="analysis-coverage">
    <summary>{en ? 'Materials used' : '本轮资料范围'} · {en ? `${coverage.read} read · ${coverage.excerpts} excerpted` : `已读 ${coverage.read} 份 · 其中节选 ${coverage.excerpts} 份`}{#if coverage.partial} · {en ? 'Partial coverage' : '含未完整读取的资料'}{/if}</summary>
    <p>{en ? `Not read this time: ${coverage.unread} · Unavailable: ${coverage.unavailable} · Additional indexed files: ${coverage.omitted}` : `本轮未读取 ${coverage.unread} 份 · 尚不可读取 ${coverage.unavailable} 份 · 另有 ${coverage.omitted} 份未纳入目录摘要`}</p>
    <p>{en ? 'Unchanged material may use its index. Findings are limited to the evidence actually included.' : '未变化的资料可沿用索引。结论以实际纳入的证据为限，未读内容不会视为已经核实。'}</p>
    <ul>{#each coverage.files as file}<li><span>{file.path}</span><small>{file.label}</small></li>{/each}</ul>
  </details>
{/if}
<style>
  .coverage{min-width:0;border-top:1px solid var(--color-border);padding-top:12px;margin-top:12px;font-size:13px;color:var(--color-muted);line-height:1.65}.coverage summary{cursor:pointer;overflow-wrap:anywhere}.coverage p{margin:8px 0}.coverage ul{list-style:none;padding:0;margin:8px 0;max-height:260px;overflow:auto;scrollbar-gutter:stable}.coverage li{display:flex;align-items:baseline;justify-content:space-between;gap:16px;padding:5px 0}.coverage li span{overflow-wrap:anywhere;min-width:0}.coverage small{font-size:inherit;flex-shrink:0}@media(max-width:600px){.coverage li{flex-direction:column;gap:0}}
</style>
