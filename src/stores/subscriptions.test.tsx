import { describe, expect, it, vi, beforeEach } from 'vitest';
vi.mock('@/services/api', () => ({
  listConversations: vi.fn(async () => []), listProjects: vi.fn(async () => []),
  getMessages: vi.fn(async () => []), isGenerating: vi.fn(async () => false),
  getConversation: vi.fn(), sendMessage: vi.fn(), stopGeneration: vi.fn(),
  retryMessage: vi.fn(), continueMessage: vi.fn(), addAttachments: vi.fn(),
  addImageAttachment: vi.fn(), removeAttachment: vi.fn(), listAttachments: vi.fn(async () => []),
  createConversation: vi.fn(),
}));
import { act, renderHook } from '@testing-library/react';
import { useConversationStore } from '@/stores/useConversationStore';

/**
 * Guards the fix for the sidebar re-rendering on every streamed frame: a
 * component that selects only sidebar state must not wake when `streams`
 * changes.
 */
describe('store subscriptions', () => {
  beforeEach(() => {
    useConversationStore.setState({ streams: {}, generating: [], conversations: [] });
  });

  it('selecting sidebar state does not re-render when streamed text changes', () => {
    let renders = 0;
    renderHook(() => {
      renders++;
      return useConversationStore((s) => s.conversations);
    });
    const base = renders;

    // Simulate ten frames of arriving text.
    act(() => {
      for (let i = 0; i < 10; i++) {
        useConversationStore.setState({
          streams: { m1: { text: 'x'.repeat(i), thinking: '' } },
        });
      }
    });
    expect(renders).toBe(base);
  });

  it('but a component selecting streams does re-render', () => {
    let renders = 0;
    renderHook(() => {
      renders++;
      return useConversationStore((s) => s.streams);
    });
    const base = renders;
    act(() => {
      useConversationStore.setState({ streams: { m1: { text: 'a', thinking: '' } } });
    });
    // Proves the first assertion is a real result, not a store that never
    // notifies anyone.
    expect(renders).toBeGreaterThan(base);
  });
});
