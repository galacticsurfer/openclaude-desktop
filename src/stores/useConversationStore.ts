import { create } from 'zustand';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useSettingsStore, defaultModel } from './useSettingsStore';
import { useUIStore } from './useUIStore';
import type {
  Attachment,
  Conversation,
  ConversationSummary,
  ListScope,
  Message,
  ProjectSummary,
  StreamDeltaEvent,
  StreamEndEvent,
  ThinkingUpdateEvent,
  ToolCallRecord,
  ToolUpdateEvent,
} from '@/types';

/** Text accumulated for a reply that is still arriving. */
export interface StreamBuffer {
  text: string;
  thinking: string;
}

interface ConversationState {
  conversations: ConversationSummary[];
  projects: ProjectSummary[];
  currentId: string | null;
  current: Conversation | null;
  messages: Message[];

  /** messageId -> text arriving right now. Empty when nothing is streaming. */
  streams: Record<string, StreamBuffer>;
  /** conversationIds with a generation in flight. */
  generating: string[];
  /**
   * messageId -> reasoning state, present only while Claude is thinking.
   *
   * Deliberately not part of `streams`: that record is rebuilt wholesale
   * from the delta buffer on every animation frame, which would drop
   * anything written here in between.
   */
  thinkingState: Record<string, { tokens: number | null }>;
  /** messageId -> tool calls in flight, while the reply is live. */
  toolState: Record<string, ToolCallRecord[]>;

  loadingList: boolean;
  loadingMessages: boolean;

  /** Files staged in the composer, not yet attached to any message. */
  pendingAttachments: Attachment[];

  loadConversations: (scope?: ListScope, projectId?: string | null) => Promise<void>;
  loadProjects: () => Promise<void>;
  open: (id: string) => Promise<void>;
  /** Show no conversation — the last tab was closed. */
  clearCurrent: () => void;
  newConversation: (projectId?: string | null) => Promise<string>;
  send: (text: string) => Promise<void>;
  stop: () => Promise<void>;
  retry: () => Promise<void>;
  editAndResend: (messageId: string, text: string) => Promise<void>;
  continueReply: () => Promise<void>;
  reloadMessages: () => Promise<void>;

  stagePaths: (paths: string[]) => Promise<void>;
  stageImage: (filename: string, mimeType: string, base64: string) => Promise<void>;
  unstage: (id: string) => Promise<void>;
  clearStaged: () => void;

  applyDelta: (e: StreamDeltaEvent) => void;
  applyThinking: (e: ThinkingUpdateEvent) => void;
  applyTool: (e: ToolUpdateEvent) => void;
  applyEnd: (e: StreamEndEvent) => void;
  markStarted: (conversationId: string, messageId: string) => void;
  patchTitle: (conversationId: string, title: string) => void;

  isGenerating: (conversationId?: string | null) => boolean;
}

/** A copy of `record` without `key`, or `record` itself when absent. */
function omit<T>(record: Record<string, T>, key: string): Record<string, T> {
  if (!(key in record)) return record;
  const next = { ...record };
  delete next[key];
  return next;
}

// --- delta coalescing -----------------------------------------------------
//
// Tokens arrive far faster than 60fps. Writing each one into the store would
// re-render (and re-parse Markdown for) the whole message per token. Instead
// deltas land in this buffer and are flushed once per animation frame.

const pending = new Map<string, StreamBuffer>();
let frame: number | null = null;

function scheduleFlush(apply: () => void) {
  if (frame !== null) return;
  frame = requestAnimationFrame(() => {
    frame = null;
    apply();
  });
}

export const useConversationStore = create<ConversationState>((set, get) => ({
  conversations: [],
  projects: [],
  currentId: null,
  current: null,
  messages: [],
  streams: {},
  generating: [],
  thinkingState: {},
  toolState: {},
  loadingList: true,
  loadingMessages: false,
  pendingAttachments: [],

  async loadConversations(scope, projectId) {
    const ui = useUIStore.getState();
    set({ loadingList: true });
    try {
      const conversations = await api.listConversations(
        scope ?? ui.scope,
        projectId !== undefined ? projectId : ui.activeProjectId,
      );
      set({ conversations, loadingList: false });
    } catch (err) {
      set({ loadingList: false });
      useUIStore.getState().toast('error', AppError.from(err).message);
    }
  },

  async loadProjects() {
    try {
      set({ projects: await api.listProjects() });
    } catch {
      /* the sidebar still works without project grouping */
    }
  },

  async open(id) {
    // Opening a conversation is what puts it in a tab; the strip never
    // decides on its own what is open.
    useUIStore.getState().openInTab(id, get().currentId);
    set({ currentId: id, loadingMessages: true, pendingAttachments: [] });
    try {
      const [current, messages] = await Promise.all([
        api.getConversation(id),
        api.getMessages(id),
      ]);
      // Guard against a slower earlier load landing after a newer one.
      if (get().currentId !== id) return;
      set({ current, messages, loadingMessages: false });
      void useSettingsStore.getState().set('ui.lastConversationId', id);

      // A generation may still be running from before this view mounted.
      if (await api.isGenerating(id)) {
        set((s) => ({
          generating: s.generating.includes(id) ? s.generating : [...s.generating, id],
        }));
      }
    } catch (err) {
      if (get().currentId !== id) return;
      set({ loadingMessages: false });
      useUIStore.getState().toast('error', AppError.from(err).message);
    }
  },

  clearCurrent() {
    set({ currentId: null, current: null, messages: [] });
  },

  async newConversation(projectId) {
    const ui = useUIStore.getState();
    const pid = projectId !== undefined ? projectId : ui.activeProjectId;
    const project = pid ? get().projects.find((p) => p.id === pid) : undefined;

    const conversation = await api.createConversation({
      model: project?.defaultModel ?? defaultModel(),
      projectId: pid,
    });
    await get().loadConversations();
    await get().open(conversation.id);
    return conversation.id;
  },

  async send(text) {
    const id = get().currentId;
    // Returning quietly here would be a silent data loss: the composer has
    // already cleared optimistically, so the typed message would just vanish.
    if (!id) {
      throw new AppError({
        kind: 'invalid',
        message: 'Open or start a conversation first.',
        retryable: false,
      });
    }

    const attachmentIds = get().pendingAttachments.map((a) => a.id);
    set({ pendingAttachments: [] });

    try {
      await api.sendMessage(id, text, attachmentIds);
      // The user turn and the assistant placeholder now exist server-side.
      await get().reloadMessages();
      await get().loadConversations();
    } catch (err) {
      const e = AppError.from(err);
      useUIStore.getState().toast('error', e.message);
      // Give the attachments back so the message can be retried as typed.
      if (attachmentIds.length > 0) {
        try {
          set({ pendingAttachments: await api.listAttachments(id) });
        } catch {
          /* ignore */
        }
      }
      throw e;
    }
  },

  async stop() {
    const id = get().currentId;
    if (!id) return;
    try {
      await api.stopGeneration(id);
    } catch {
      /* already finished */
    }
  },

  async retry() {
    const id = get().currentId;
    if (!id) return;
    try {
      await api.retryMessage(id);
      await get().reloadMessages();
    } catch (err) {
      useUIStore.getState().toast('error', AppError.from(err).message);
    }
  },

  async editAndResend(messageId, text) {
    const id = get().currentId;
    if (!id) return;
    try {
      await api.editAndResend(id, messageId, text);
      await get().reloadMessages();
    } catch (err) {
      useUIStore.getState().toast('error', AppError.from(err).message);
    }
  },

  async continueReply() {
    const id = get().currentId;
    if (!id) return;
    try {
      await api.continueMessage(id);
      await get().reloadMessages();
    } catch (err) {
      useUIStore.getState().toast('error', AppError.from(err).message);
    }
  },

  async reloadMessages() {
    const id = get().currentId;
    if (!id) return;
    try {
      const messages = await api.getMessages(id);
      if (get().currentId !== id) return;
      set({ messages });
    } catch {
      /* keep what is on screen rather than blanking it */
    }
  },

  async stagePaths(paths) {
    if (paths.length === 0) return;
    const conversationId = get().currentId;
    try {
      const result = await api.addAttachments(paths, conversationId);
      set((s) => ({ pendingAttachments: [...s.pendingAttachments, ...result.added] }));
      for (const r of result.rejected) {
        useUIStore.getState().toast('error', `${r.filename}: ${r.reason}`);
      }
    } catch (err) {
      useUIStore.getState().toast('error', AppError.from(err).message);
    }
  },

  async stageImage(filename, mimeType, base64) {
    try {
      const a = await api.addImageAttachment(filename, mimeType, base64, get().currentId);
      set((s) => ({ pendingAttachments: [...s.pendingAttachments, a] }));
    } catch (err) {
      useUIStore.getState().toast('error', AppError.from(err).message);
    }
  },

  async unstage(id) {
    set((s) => ({ pendingAttachments: s.pendingAttachments.filter((a) => a.id !== id) }));
    try {
      await api.removeAttachment(id);
    } catch {
      /* the row is gone from the composer either way */
    }
  },

  clearStaged: () => set({ pendingAttachments: [] }),

  markStarted(conversationId, messageId) {
    pending.set(messageId, { text: '', thinking: '' });
    set((s) => ({
      generating: s.generating.includes(conversationId)
        ? s.generating
        : [...s.generating, conversationId],
      streams: { ...s.streams, [messageId]: { text: '', thinking: '' } },
      // A retry reuses the message id; clear any reasoning left from the
      // attempt before it.
      thinkingState: omit(s.thinkingState, messageId),
      toolState: omit(s.toolState, messageId),
    }));
  },

  applyDelta(e) {
    const buf = pending.get(e.messageId) ?? { text: '', thinking: '' };
    if (e.channel === 'thinking') buf.thinking += e.text;
    else buf.text += e.text;
    pending.set(e.messageId, buf);

    scheduleFlush(() => {
      const next: Record<string, StreamBuffer> = { ...get().streams };
      for (const [id, b] of pending) next[id] = { ...b };
      set({ streams: next });
    });
  },

  applyThinking(e) {
    // Only ever set while the reply is live; the persisted count on the
    // message is what survives afterwards.
    set((s) => ({
      thinkingState: { ...s.thinkingState, [e.messageId]: { tokens: e.tokens } },
    }));
  },

  applyTool(e) {
    set((s) => {
      const list = s.toolState[e.messageId] ?? [];
      const at = list.findIndex((t) => t.id === e.id);
      // Merge rather than replace: the call is announced first without
      // arguments, then again with them, then once more with its outcome.
      const prev = at === -1 ? undefined : list[at];
      const entry: ToolCallRecord = {
        ...prev,
        id: e.id,
        name: e.name || (prev?.name ?? ''),
        ...(e.input === null || e.input === undefined ? {} : { input: e.input }),
        ...(e.ok === null ? {} : { ok: e.ok }),
      };
      const next = at === -1 ? [...list, entry] : list.map((t, i) => (i === at ? entry : t));
      return { toolState: { ...s.toolState, [e.messageId]: next } };
    });
  },

  applyEnd(e) {
    pending.delete(e.messageId);
    set((s) => {
      const streams = { ...s.streams };
      delete streams[e.messageId];
      return {
        streams,
        thinkingState: omit(s.thinkingState, e.messageId),
        toolState: omit(s.toolState, e.messageId),
        generating: s.generating.filter((id) => id !== e.conversationId),
      };
    });
    // Pull the authoritative row: it carries the final status, usage and any
    // error, and is what a later reload would show anyway.
    if (get().currentId === e.conversationId) void get().reloadMessages();
  },

  patchTitle(conversationId, title) {
    set((s) => ({
      conversations: s.conversations.map((c) =>
        c.id === conversationId ? { ...c, title } : c,
      ),
      current:
        s.current && s.current.id === conversationId ? { ...s.current, title } : s.current,
    }));
  },

  isGenerating(conversationId) {
    const id = conversationId ?? get().currentId;
    return id ? get().generating.includes(id) : false;
  },
}));
