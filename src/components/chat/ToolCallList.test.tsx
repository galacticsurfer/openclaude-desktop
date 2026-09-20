import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { ToolCallList } from './ToolCallList';

describe('ToolCallList', () => {
  it('hides arguments until asked, then shows them', async () => {
    render(
      <ToolCallList
        calls={[{ id: '1', name: 'mcp__notes__search', input: { query: 'quarterly report' }, ok: true }]}
      />,
    );
    expect(screen.queryByText('quarterly report')).toBeNull();
    await userEvent.click(screen.getByRole('button'));
    expect(screen.getByText('quarterly report')).toBeTruthy();
    expect(screen.getByText('query')).toBeTruthy();
  });

  it('is not expandable when a call took no arguments', () => {
    render(<ToolCallList calls={[{ id: '1', name: 'mcp__clock__now', ok: true }]} />);
    expect(screen.getByRole('button')).toHaveProperty('disabled', true);
  });

  it('marks a call still running, and a failed one', () => {
    render(
      <ToolCallList
        calls={[
          { id: '1', name: 'a' },
          { id: '2', name: 'b', ok: false },
        ]}
      />,
    );
    expect(screen.getByText('running…')).toBeTruthy();
    expect(screen.getByLabelText('failed')).toBeTruthy();
  });

  it('truncates a value long enough to swamp the transcript', async () => {
    render(
      <ToolCallList calls={[{ id: '1', name: 't', input: { body: 'x'.repeat(5000) }, ok: true }]} />,
    );
    await userEvent.click(screen.getByRole('button'));
    const shown = screen.getByText(/x+…/).textContent ?? '';
    expect(shown.length).toBeLessThan(400);
    expect(shown.endsWith('…')).toBe(true);
  });
});
