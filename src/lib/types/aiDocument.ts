export type AiBasis = 'fact' | 'inference' | 'suggestion' | 'unknown';
export type AiCitation = { source_id: string; quote: string };
export type AiStatement = { text: string; basis: AiBasis; citations: AiCitation[] };
export type AiBlock =
  | { type: 'paragraph'; content: AiStatement }
  | { type: 'bullets'; items: AiStatement[] }
  | { type: 'numbered'; items: AiStatement[] }
  | { type: 'table'; columns: string[]; rows: AiStatement[][] };
export type AiDocumentValue = {
  schema_version: 'msl.readable.v1';
  title: string;
  sections: { title: string; blocks: AiBlock[] }[];
};

export function aiDocumentText(document: AiDocumentValue): string {
  const statement = (item: AiStatement) => `${item.basis === 'fact' ? '' : ({inference:'[推断] ', suggestion:'[建议] ', unknown:'[待核实] '}[item.basis])}${item.text}`;
  const lines = [document.title];
  for (const section of document.sections) {
    lines.push('', section.title);
    for (const block of section.blocks) {
      if (block.type === 'paragraph') lines.push(statement(block.content));
      else if (block.type === 'bullets') lines.push(...block.items.map(item => `- ${statement(item)}`));
      else if (block.type === 'numbered') lines.push(...block.items.map((item, index) => `${index + 1}. ${statement(item)}`));
      else {
        lines.push(block.columns.join(' | '));
        lines.push(...block.rows.map(row => row.map(statement).join(' | ')));
      }
    }
  }
  return lines.join('\n');
}
