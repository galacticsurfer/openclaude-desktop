import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useConversationStore } from '@/stores/useConversationStore';
import type {
  StreamDeltaEvent,
  StreamEndEvent,
  StreamStartEvent,
  ThinkingUpdateEvent,
} from '@/types';

/**
 * Bridge the backend's streaming events into the store.
 *
 * Mounted once, at the app root — the listeners must survive switching
 * conversations, because a generation keeps running when you navigate away
 * from it.
 */
export function useStreamEvents(): void {
  useEffect(() => {
    const store = useConversationStore.getState();
    const unlisteners: Array<() => void> = [];
    let disposed = false;

    const register = (p: Promise<() => void>) => {
      void p.then((un) => {
        // The effect may have been torn down while `listen` was in flight.
        if (disposed) un();
        else unlisteners.push(un);
      });
    };

    register(
      listen<StreamStartEvent>('chat:start', (e) =>
        store.markStarted(e.payload.conversationId, e.payload.messageId),
      ),
    );
    register(listen<StreamDeltaEvent>('chat:delta', (e) => store.applyDelta(e.payload)));
    register(
      listen<ThinkingUpdateEvent>('chat:thinking', (e) => store.applyThinking(e.payload)),
    );
    register(listen<StreamEndEvent>('chat:end', (e) => store.applyEnd(e.payload)));

    register(
      listen<null>('tray:new-conversation', () => {
        void useConversationStore.getState().newConversation();
      }),
    );
    register(
      listen<string>('conversation:updated', () => {
        void useConversationStore.getState().loadConversations();
      }),
    );
    register(
      listen<[string, string]>('conversation:title', (e) => {
        const [id, title] = e.payload;
        useConversationStore.getState().patchTitle(id, title);
      }),
    );

    return () => {
      disposed = true;
      for (const un of unlisteners) un();
    };
  }, []);
}
