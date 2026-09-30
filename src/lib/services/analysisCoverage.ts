export function analysisCoverage(raw: string | null | undefined, locale: string) {
  let value: Record<string, unknown>;
  try { value = JSON.parse(raw ?? '{}'); } catch { return null; }
  if (!value || typeof value !== 'object' || typeof value.documents_read !== 'number') return null;
  const count = (key: string) => typeof value[key] === 'number' && Number.isFinite(value[key]) ? Math.max(0, Math.floor(value[key] as number)) : 0;
  const labels: Record<string, [string, string]> = { read: ['已读取', 'Read'], excerpt: ['节选', 'Excerpt'], unread: ['本轮未读取', 'Not read this time'], indexed: ['沿用索引，未重读正文', 'Index only'], unavailable: ['尚不可读取', 'Unavailable'] };
  const files = (Array.isArray(value.document_coverage) ? value.document_coverage : []).filter((row): row is Record<string, unknown> => row !== null && typeof row === 'object' && typeof row.path === 'string').map(row => ({ path: String(row.path), label: (labels[String(row.status)] ?? labels.unread)[locale === 'en-US' ? 1 : 0] }));
  const read = count('documents_read'), excerpts = count('documents_excerpted'), unread = count('documents_unread'), unavailable = count('documents_unavailable'), omitted = count('documents_metadata_omitted');
  return { read, excerpts, unread, unavailable, omitted, partial: excerpts + unread + unavailable + omitted > 0, files };
}
