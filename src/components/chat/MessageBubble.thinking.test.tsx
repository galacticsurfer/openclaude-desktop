import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { MessageBubble } from './MessageBubble';
import type { Message } from '@/types';

vi.mock('@/components/markdown/Markdown', () => ({
  Markdown: ({ children }: { children: string }) => <div>{children}</div>,
}));

const message = (over: Partial<Message> = {}): Message => ({
  id: 'm1',
  conversationId: 'c1',
  seq: 1,
  role: 'assistant',
  content: '',
  thinking: null,
  thinkingTokens: null,
  status: 'complete',
  model: null,
  providerMessageId: null,
  stopReason: null,
  inputTokens: null,
  outputTokens: null,
  cacheReadTokens: null,
  cacheWriteTokens: null,
  errorKind: null,
  errorMessage: null,
  createdAt: 0,
  updatedAt: 0,
  metadata: {},
  attachments: [],
  ...over,
});

const props = {
  isLast: true,
  onRetry: () => {},
  onContinue: () => {},
  onBranch: () => {},
  onCopy: () => {},
};

describe('reasoning indicator', () => {
  it('says Claude is thinking before any estimate has arrived', () => {
    render(
      <MessageBubble
        {...props}
        message={message()}
        stream={{ text: '', thinking: '' }}
        thinkingState={{ tokens: null }}
      />,
    );
    expect(screen.getByText(/Thinking/)).toBeTruthy();
  });

  it('shows the running token estimate once the CLI reports one', () => {
    render(
      <MessageBubble
        {...props}
        message={message()}
        stream={{ text: '', thinking: '' }}
        thinkingState={{ tokens: 1200 }}
      />,
    );
    expect(screen.getByText(/~1\.2k tokens/)).toBeTruthy();
  });

  it('keeps a summary on the finished message, from the persisted count', () => {
    render(
      <MessageBubble
        {...props}
        message={message({ content: 'Done.', thinkingTokens: 50 })}
        stream={null}
        thinkingState={null}
      />,
    );
    expect(screen.getByText(/Thought for ~50 tokens/)).toBeTruthy();
  });

  it('shows nothing when the reply did no reasoning', () => {
    render(
      <MessageBubble
        {...props}
        message={message({ content: 'Hi.' })}
        stream={null}
        thinkingState={null}
      />,
    );
    expect(screen.queryByText(/Think/)).toBeNull();
  });
});
