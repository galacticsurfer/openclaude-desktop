import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Markdown } from './Markdown';

// The code block pulls in Shiki and Tauri plugins; neither belongs in a unit
// test of the Markdown mapping.
vi.mock('./CodeBlock', () => ({
  CodeBlock: ({ code, language }: { code: string; language?: string }) => (
    <pre data-testid="code-block" data-language={language}>
      {code}
    </pre>
  ),
}));

describe('Markdown', () => {
  it('renders GFM structure: headings, lists and tables', () => {
    render(
      <Markdown>{`# Title

- one
- two

| a | b |
| --- | --- |
| 1 | 2 |`}</Markdown>,
    );
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Title');
    expect(screen.getAllByRole('listitem')).toHaveLength(2);
    expect(screen.getByRole('table')).toBeInTheDocument();
  });

  it('routes fenced blocks to the code block with their language', () => {
    render(<Markdown>{'```python\nprint(1)\n```'}</Markdown>);
    const block = screen.getByTestId('code-block');
    expect(block).toHaveAttribute('data-language', 'python');
    expect(block).toHaveTextContent('print(1)');
  });

  it('keeps inline code inline rather than promoting it to a block', () => {
    const { container } = render(<Markdown>{'use `npm run dev` now'}</Markdown>);
    expect(screen.queryByTestId('code-block')).toBeNull();
    expect(container.querySelector('code')).toHaveTextContent('npm run dev');
  });

  it('never renders raw HTML from model output', () => {
    // rehype-raw is deliberately not installed; this is the guarantee.
    const { container } = render(
      <Markdown>{'<script>alert(1)</script><img src=x onerror=alert(1)>'}</Markdown>,
    );
    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('alert(1)');
  });

  it('wraps tables so wide content scrolls inside its own container', () => {
    const { container } = render(
      <Markdown>{'| a | b |\n| --- | --- |\n| 1 | 2 |'}</Markdown>,
    );
    expect(container.querySelector('.table-wrap')).toBeInTheDocument();
  });

  it('renders task lists as read-only checkboxes', () => {
    render(<Markdown>{'- [x] done\n- [ ] todo'}</Markdown>);
    const boxes = screen.getAllByRole('checkbox') as HTMLInputElement[];
    expect(boxes).toHaveLength(2);
    expect(boxes[0]!.checked).toBe(true);
    expect(boxes[0]!.readOnly).toBe(true);
  });
});
