<script lang="ts">
  import { locale, t } from '$lib/i18n';

  type Document = { id: number; relative_path: string; extract_status: string; error_message: string | null };
  let { documents }: { documents: Document[] } = $props();
  let expanded = $state(false);
  const en = $derived($locale === 'en-US');
  const remaining = $derived(documents.slice(8));
  const statusLabels: Record<string, [string, string]> = {
    ready: ['可用于分析', 'Available for analysis'],
    pending: ['等待读取', 'Waiting to be read'],
    unsupported: ['暂不支持此格式', 'Format not supported'],
    needs_ocr: ['需要文字识别', 'Text recognition needed'],
    too_large: ['文件超出读取上限', 'File exceeds the reading limit'],
    failed_parse: ['内容读取失败', 'Could not read the contents'],
    failed_encoding: ['文字编码无法识别', 'Text encoding not recognized'],
    failed: ['内容读取失败', 'Could not read the contents'],
  };
  const statusLabel = (status: string) => statusLabels[status]?.[en ? 1 : 0]
    ?? (en ? 'Reading status needs checking' : '读取状态待确认');
</script>

{#snippet row(document: Document)}
  <div class="doc-row" data-testid={`workspace-document-${document.id}`}>
    <span class="doc-name">{document.relative_path}</span>
    <span class="doc-status"><span>{statusLabel(document.extract_status)}</span>{#if document.error_message}<small>{document.error_message}</small>{/if}</span>
  </div>
{/snippet}

<div class="doc-list">
  {#each documents.slice(0, 8) as document (document.id)}{@render row(document)}{/each}
  {#if remaining.length}
    <details class="more-documents" data-testid="workspace-more-documents" bind:open={expanded}>
      <summary>{expanded
        ? (en ? `Hide ${remaining.length} additional documents` : `收起其余 ${remaining.length} 份资料`)
        : (en ? `Show ${remaining.length} more documents` : `查看其余 ${remaining.length} 份资料`)}</summary>
      <div class="remaining-list">{#each remaining as document (document.id)}{@render row(document)}{/each}</div>
    </details>
  {/if}
  {#if !documents.length}<p class="empty">{t('workspace.noSupportedDocuments', {}, $locale)}</p>{/if}
</div>

<style>
  .doc-list,.remaining-list{display:grid;gap:10px;min-width:0}.doc-list{margin-top:16px}
  .doc-row{display:grid;grid-template-columns:minmax(0,1fr) minmax(140px,40%);gap:16px;border-top:1px solid var(--color-border);padding-top:10px;font-size:15px;line-height:1.65;min-width:0}
  .doc-name,.doc-status{min-width:0;overflow-wrap:anywhere}.doc-status{display:grid;gap:3px;color:var(--color-muted);align-content:start}.doc-status small{font:inherit;font-size:14px}
  .more-documents{min-width:0}.more-documents summary{cursor:pointer;color:var(--color-primary);font-size:15px;line-height:1.6;padding:10px 0;overflow-wrap:anywhere}.more-documents summary:focus-visible{outline:2px solid var(--color-primary);outline-offset:3px;border-radius:4px}
  .empty{margin:0;color:var(--color-muted);font-size:15px;line-height:1.65}
  @media(max-width:640px){.doc-row{grid-template-columns:minmax(0,1fr);gap:4px}}
</style>
