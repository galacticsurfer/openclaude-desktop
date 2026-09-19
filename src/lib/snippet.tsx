import type { ReactNode } from 'react';

/**
 * Render an FTS5 snippet, which delimits matches with `<<` and `>>`.
 *
 * Built as React nodes rather than HTML so a match inside user content can
 * never inject markup.
 */
export function renderSnippet(snippet: string): ReactNode[] {
  const out: ReactNode[] = [];
  let rest = snippet;
  let key = 0;

  while (rest.length > 0) {
    const open = rest.indexOf('<<');
    if (open === -1) {
      out.push(rest);
      break;
    }
    const close = rest.indexOf('>>', open + 2);
    if (close === -1) {
      out.push(rest);
      break;
    }
    if (open > 0) out.push(rest.slice(0, open));
    out.push(
      <mark key={key++} className="hit">
        {rest.slice(open + 2, close)}
      </mark>,
    );
    rest = rest.slice(close + 2);
  }

  return out;
}
