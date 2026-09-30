import { Lexer, type Token } from 'marked';

// Reading-only projection. Never write this back to records or render it as HTML.
// Svelte escapes the resulting text; model-supplied links/images are not loaded.
function tokensText(tokens: Token[] = []): string {
  return tokens.map(token => {
    switch (token.type) {
      case 'space': return '\n';
      case 'hr': return '\n';
      case 'br': return '\n';
      case 'checkbox': return '';
      case 'code': return token.text + '\n';
      case 'codespan': return token.text;
      case 'heading': return tokensText(token.tokens) + '\n\n';
      case 'paragraph': return tokensText(token.tokens) + '\n';
      case 'blockquote': return tokensText(token.tokens);
      case 'list': return token.items.map((item: {tokens: Token[]; task?: boolean; checked?: boolean}, index: number) => {
        const prefix = item.task ? (item.checked ? '☑ ' : '☐ ') : token.ordered ? `${Number(token.start) + index}. ` : '• ';
        return prefix + tokensText(item.tokens).trim();
      }).join('\n') + '\n';
      case 'table': return [token.header, ...token.rows].map((row: {tokens: Token[]}[]) => row.map(cell => tokensText(cell.tokens)).join(' · ')).join('\n') + '\n';
      case 'link': {
        const label = tokensText(token.tokens);
        // Keep useful source addresses as inert text. Never make unsafe links active.
        return /^(https?:|mailto:)/i.test(token.href) && label !== token.href ? `${label}（${token.href}）` : label;
      }
      case 'image': return token.text;
      default: return 'tokens' in token && token.tokens ? tokensText(token.tokens) : ('text' in token ? token.text : token.raw);
    }
  }).join('');
}

export function aiText(value: string | null | undefined): string {
  return value ? tokensText(Lexer.lex(value, {gfm:true})).trim() : '';
}

export function briefLines(value: string): string[] {
  // Old releases stripped the opening ** from headings before saving a brief.
  // Repair only that known line-boundary defect, not literal math or identifiers.
  const repaired = value.split(/\r?\n/).map(raw => {
    const line = raw.trim().replace(/^[•·▪‣]\s*/, '');
    return /^[^*`]+\*{2}$/.test(line) ? line.slice(0, -2) : line;
  }).join('\n');
  return aiText(repaired).split(/\r?\n/)
    .map(line => line.trim().replace(/^•\s+/, ''))
    .map(line => line.replace(/\s*[（(\[][^）)\]]*(?:source_type|entity_id|workspace_id|source_id|\[source_)[^）)\]]*[）)\]]/gi, '').trim())
    .filter(Boolean);
}
