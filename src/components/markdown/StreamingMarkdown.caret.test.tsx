import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { StreamingMarkdown } from './StreamingMarkdown';

describe('StreamingMarkdown caret', () => {
  it('trails the last character instead of wrapping to its own line', () => {
    const { container } = render(
      <StreamingMarkdown caret>{'Done.\n\nStill writ'}</StreamingMarkdown>,
    );
    const caret = container.querySelector('.stream-caret');
    expect(caret).not.toBeNull();
    // Inside the tail, not a sibling of it — a block-level sibling is what
    // pushed the cursor onto the next line.
    expect(caret?.parentElement?.textContent).toBe('Still writ');
  });

  it('still shows the caret when the tail is empty', () => {
    const { container } = render(<StreamingMarkdown caret>{'Done.\n\n'}</StreamingMarkdown>);
    expect(container.querySelector('.stream-caret')).not.toBeNull();
  });

  it('draws no caret when not asked for one', () => {
    const { container } = render(<StreamingMarkdown>{'Still writ'}</StreamingMarkdown>);
    expect(container.querySelector('.stream-caret')).toBeNull();
  });
});
