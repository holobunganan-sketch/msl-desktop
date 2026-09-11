export function paginate<T>(items: readonly T[], requestedPage: number, pageSize = 50) {
  const size = Number.isFinite(pageSize) ? Math.max(1, Math.floor(pageSize)) : 50;
  const total = items.length;
  const pages = Math.max(1, Math.ceil(total / size));
  const page = Math.min(pages, Math.max(1, Number.isFinite(requestedPage) ? Math.floor(requestedPage) : 1));
  const offset = (page - 1) * size;
  return { items: items.slice(offset, offset + size), total, pages, page, start: total ? offset + 1 : 0, end: Math.min(offset + size, total) };
}
