<script lang="ts">
  import type { AiDocumentValue, AiStatement } from '$lib/types/aiDocument';
  import { locale } from '$lib/i18n';
  import KnowledgeSources from './KnowledgeSources.svelte';
  import type { Pack } from '$lib/services/knowledge';
  let { document, mode = 'reading', pack = null }: { document: AiDocumentValue; mode?: 'compact' | 'reading'; pack?: Pack | null } = $props();
  let en = $derived($locale === 'en-US');
  const basis = (value: AiStatement['basis']) => ({
    fact: en ? 'Fact' : '事实', inference: en ? 'Interpretation' : '推断', suggestion: en ? 'Suggestion' : '建议', unknown: en ? 'Unknown' : '待确认'
  })[value];
</script>

<article class:compact={mode === 'compact'} class="ai-document">
  <h2>{document.title}</h2>
  {#each document.sections as section}
    <section>
      <h3>{section.title}</h3>
      {#each section.blocks as block}
        {#if block.type === 'paragraph'}
          <p><span class="basis" data-basis={block.content.basis}>{basis(block.content.basis)}</span>{block.content.text}</p>
        {:else if block.type === 'bullets'}
          <ul>{#each block.items as item}<li><span class="basis" data-basis={item.basis}>{basis(item.basis)}</span>{item.text}</li>{/each}</ul>
        {:else if block.type === 'numbered'}
          <ol>{#each block.items as item}<li><span class="basis" data-basis={item.basis}>{basis(item.basis)}</span>{item.text}</li>{/each}</ol>
        {:else}
          <div class="table-wrap"><table><thead><tr>{#each block.columns as column}<th>{column}</th>{/each}</tr></thead><tbody>{#each block.rows as row}<tr>{#each row as cell}<td><span class="basis" data-basis={cell.basis}>{basis(cell.basis)}</span>{cell.text}</td>{/each}</tr>{/each}</tbody></table></div>
        {/if}
        {@const statements = block.type === 'paragraph' ? [block.content] : block.type === 'table' ? block.rows.flat() : block.items}
        <KnowledgeSources citations={statements.flatMap(item => item.citations)} {pack}/>
      {/each}
    </section>
  {/each}
</article>

<style>
  .ai-document{width:min(100%,840px);margin-inline:auto;color:var(--color-text);font-size:calc(16px * var(--font-scale,1));line-height:1.75}.ai-document h2{margin:0 0 22px;font:500 28px var(--font-serif)}.ai-document section+section{margin-top:28px}.ai-document h3{margin:0 0 10px;font:650 17px var(--font-sans)}.ai-document p,.ai-document ul,.ai-document ol{margin:9px 0}.ai-document ul,.ai-document ol{padding-left:1.45em}.ai-document li+li{margin-top:7px}.basis{display:inline-flex;align-items:center;min-height:22px;margin-right:8px;padding:1px 7px;border-radius:999px;background:var(--color-surface-muted);color:var(--color-muted);font-size:11px;line-height:1}.basis[data-basis='suggestion']{background:var(--color-primary-soft);color:var(--color-primary)}.basis[data-basis='unknown']{background:var(--color-warning-soft);color:var(--color-warning)}.table-wrap{max-width:100%;overflow:auto;scrollbar-gutter:stable}table{min-width:560px;width:100%;border-collapse:collapse}th,td{padding:10px 12px;border-bottom:1px solid var(--color-border);text-align:left;vertical-align:top}.compact{font-size:calc(15px * var(--font-scale,1));line-height:1.65}
</style>
