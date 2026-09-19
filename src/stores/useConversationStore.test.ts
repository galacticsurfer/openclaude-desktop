import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/services/api', () => ({
  listConversations: vi.fn(async () => []),
  listProjects: vi.fn(async () => []),
  getConversation: vi.fn(),
  getMessages: vi.fn(async () => []),
  isGenerating: vi.fn(async () => false),
  sendMessage: vi.fn(async () => 'msg-1'),
  stopGeneration: vi.fn(async () => undefined),
  retryMessage: vi.fn(async () => 'msg-2'),
  continueMessage: vi.fn(async () => 'msg-3'),
  addAttachments: vi.fn(),
  addImageAttachment: vi.fn(),
  removeAttachment: vi.fn(async () => undefined),
  listAttachments: vi.fn(async () => []),
  createConversation: vi.fn(),
}));

import { useConversationStore } from './useConversationStore';
import type { StreamDeltaEvent, StreamEndEvent } from '@/types';

/*
 * The store coalesces deltas into an animation frame. The stub must defer
 * like a real one: running the callback synchronously would return the frame
 * id *after* the callback had already cleared it, wedging the scheduler — an
 * artefact of the stub, not of the store.
 */
let frames: FrameRequestCallback[] = [];
let nextFrameId = 1;
const runFrames = () => {
  const pending = frames;
  frames = [];
  for (const cb of pending) cb(performance.now());
};

beforeEach(() => {
  frames = [];
  nextFrameId = 1;
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
    frames.push(cb);
    return nextFrameId++;
  });
  useConversationStore.setState({
    streams: {},
    generating: [],
    messages: [],
    currentId: null,
    conversations: [],
  });
});

const delta = (text: string, channel: 'text' | 'thinking' = 'text'): StreamDeltaEvent => ({
  conversationId: 'c1',
  messageId: 'm1',
  text,
  channel,
});

describe('streaming buffer', () => {
  it('marks the conversation as generating when a stream starts', () => {
    const s = useConversationStore.getState();
    s.markStarted('c1', 'm1');

    expect(useConversationStore.getState().generating).toContain('c1');
    expect(useConversationStore.getState().streams['m1']).toEqual({ text: '', thinking: '' });
    expect(useConversationStore.getState().isGenerating('c1')).toBe(true);
  });

  it('accumulates deltas in order', () => {
    const s = useConversationStore.getState();
    s.markStarted('c1', 'm1');
    s.applyDelta(delta('Write-ahead '));
    s.applyDelta(delta('logging '));
    s.applyDelta(delta('batches writes.'));
    runFrames();

    expect(useConversationStore.getState().streams['m1']?.text).toBe(
      'Write-ahead logging batches writes.',
    );
  });

  it('keeps thinking output on a separate channel', () => {
    const s = useConversationStore.getState();
    s.markStarted('c1', 'm1');
    s.applyDelta(delta('considering', 'thinking'));
    s.applyDelta(delta('the answer'));
    runFrames();

    const buf = useConversationStore.getState().streams['m1'];
    expect(buf?.thinking).toBe('considering');
    expect(buf?.text).toBe('the answer');
  });

  it('clears the buffer and the generating flag when the stream ends', () => {
    const s = useConversationStore.getState();
    s.markStarted('c1', 'm1');
    s.applyDelta(delta('partial'));
    runFrames();

    const end: StreamEndEvent = {
      conversationId: 'c1',
      messageId: 'm1',
      status: 'complete',
      stopReason: 'end_turn',
      inputTokens: 10,
      outputTokens: 20,
      error: null,
    };
    s.applyEnd(end);

    const state = useConversationStore.getState();
    expect(state.streams['m1']).toBeUndefined();
    expect(state.generating).not.toContain('c1');
    expect(state.isGenerating('c1')).toBe(false);
  });

  it('tracks two conversations streaming at once independently', () => {
    const s = useConversationStore.getState();
    s.markStarted('c1', 'm1');
    s.markStarted('c2', 'm2');
    s.applyDelta({ conversationId: 'c1', messageId: 'm1', text: 'one', channel: 'text' });
    s.applyDelta({ conversationId: 'c2', messageId: 'm2', text: 'two', channel: 'text' });
    runFrames();

    const state = useConversationStore.getState();
    expect(state.streams['m1']?.text).toBe('one');
    expect(state.streams['m2']?.text).toBe('two');

    s.applyEnd({
      conversationId: 'c1', messageId: 'm1', status: 'complete',
      stopReason: null, inputTokens: null, outputTokens: null, error: null,
    });

    // Ending one must not disturb the other.
    expect(useConversationStore.getState().streams['m2']?.text).toBe('two');
    expect(useConversationStore.getState().generating).toEqual(['c2']);
  });

  it('a delta arriving after the end is ignored rather than resurrecting the buffer', () => {
    const s = useConversationStore.getState();
    s.markStarted('c1', 'm1');
    s.applyEnd({
      conversationId: 'c1', messageId: 'm1', status: 'interrupted',
      stopReason: null, inputTokens: null, outputTokens: null, error: null,
    });
    s.applyDelta(delta('late token'));
    runFrames();

    // The message row is authoritative once the stream has ended; a late
    // delta must not put the bubble back into a streaming state.
    expect(useConversationStore.getState().generating).not.toContain('c1');
  });
});

describe('title patching', () => {
  it('updates the sidebar row and the open conversation together', () => {
    useConversationStore.setState({
      conversations: [{ id: 'c1', title: 'New conversation' }] as never,
      current: { id: 'c1', title: 'New conversation' } as never,
    });

    useConversationStore.getState().patchTitle('c1', 'Debug Mongo timeout');

    const state = useConversationStore.getState();
    expect(state.conversations[0]?.title).toBe('Debug Mongo timeout');
    expect(state.current?.title).toBe('Debug Mongo timeout');
  });

  it('leaves other conversations alone', () => {
    useConversationStore.setState({
      conversations: [
        { id: 'c1', title: 'A' },
        { id: 'c2', title: 'B' },
      ] as never,
      current: null,
    });
    useConversationStore.getState().patchTitle('c1', 'Renamed');
    expect(useConversationStore.getState().conversations[1]?.title).toBe('B');
  });
});
