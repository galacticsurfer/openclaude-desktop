import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { renderSnippet } from './snippet';

describe('renderSnippet', () => {
  it('highlights the matched terms FTS5 delimited', () => {
    render(<p>{renderSnippet('the <<pool>> is exhausted')}</p>);
    const mark = screen.getByText('pool');
    expect(mark.tagName).toBe('MARK');
    expect(screen.getByText(/is exhausted/)).toBeInTheDocument();
  });

  it('handles several matches in one snippet', () => {
    const { container } = render(<p>{renderSnippet('<<a>> and <<b>> and c')}</p>);
    expect(container.querySelectorAll('mark')).toHaveLength(2);
  });

  it('renders markup in the source text as literal characters, never as HTML', () => {
    // Snippets contain user and model text; an injected tag must not become
    // an element.
    const { container } = render(
      <p>{renderSnippet('<<hit>> <img src=x onerror=alert(1)>')}</p>,
    );
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('<img src=x onerror=alert(1)>');
  });

  it('passes through text with no delimiters', () => {
    const { container } = render(<p>{renderSnippet('plain text')}</p>);
    expect(container.querySelectorAll('mark')).toHaveLength(0);
    expect(container.textContent).toBe('plain text');
  });

  it('does not hang on an unbalanced delimiter', () => {
    const { container } = render(<p>{renderSnippet('broken <<open')}</p>);
    expect(container.textContent).toBe('broken <<open');
  });
});
