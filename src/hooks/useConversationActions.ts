import { useCallback } from 'react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useConversationStore } from '@/stores/useConversationStore';
import { useUIStore } from '@/stores/useUIStore';
import type { ConversationSummary, ExportFormat } from '@/types';
import type { ConversationAction } from '@/components/sidebar/ConversationItem';

/**
 * The conversation context-menu actions, shared by the sidebar and the
 * command palette so both behave identically.
 */
export function useConversationActions() {
  const { loadConversations, open, currentId } = useConversationStore();
  const { toast, confirm, openOverlay } = useUIStore();

  return useCallback(
    async (action: ConversationAction, c: ConversationSummary) => {
      const refresh = () => loadConversations();

      try {
        switch (action) {
          case 'rename': {
            const title = window.prompt('Rename conversation', c.title);
            if (title === null || title.trim() === '' || title === c.title) return;
            await api.renameConversation(c.id, title.trim());
            await refresh();
            break;
          }

          case 'pin':
            await api.setConversationPinned(c.id, !c.pinned);
            await refresh();
            break;

          case 'archive':
            await api.setConversationArchived(c.id, !c.archived);
            await refresh();
            toast('info', c.archived ? 'Unarchived.' : 'Archived.');
            break;

          case 'trash':
            // Soft delete, so this needs no confirmation — it is reversible,
            // and the toast offers the undo directly.
            await api.trashConversation(c.id);
            await refresh();
            toast('info', `"${truncate(c.title)}" moved to trash.`, {
              label: 'Undo',
              run: () => {
                void api.restoreConversation(c.id).then(refresh);
              },
            });
            break;

          case 'restore':
            await api.restoreConversation(c.id);
            await refresh();
            break;

          case 'deleteForever':
            confirm({
              title: 'Delete permanently?',
              body: `"${c.title}" and all of its messages will be removed from this computer. This cannot be undone.`,
              confirmLabel: 'Delete permanently',
              destructive: true,
              onConfirm: async () => {
                await api.deleteConversationPermanently(c.id);
                await refresh();
                toast('info', 'Conversation deleted.');
              },
            });
            break;

          case 'duplicate': {
            const copy = await api.duplicateConversation(c.id);
            await refresh();
            await open(copy.id);
            break;
          }

          case 'move':
            openOverlay({ kind: 'conversationInfo', conversationId: c.id });
            break;

          case 'export':
            await exportConversation(c.id, 'markdown', toast);
            break;
        }
      } catch (err) {
        toast('error', AppError.from(err).message);
      }

      // Keep the open conversation's header in step with a rename or a move.
      if (currentId === c.id) void useConversationStore.getState().open(c.id);
    },
    [loadConversations, open, currentId, toast, confirm, openOverlay],
  );
}

export async function exportConversation(
  id: string,
  format: ExportFormat,
  toast: (kind: 'info' | 'error' | 'success', message: string) => void,
): Promise<void> {
  try {
    const result = await api.exportConversation(id, format);
    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({
      defaultPath: result.filename,
      filters: [{ name: format, extensions: [result.filename.split('.').pop() ?? 'txt'] }],
    });
    if (!path) return;
    await api.writeTextFile(path, result.content);
    toast('success', 'Conversation exported.');
  } catch (err) {
    toast('error', AppError.from(err).message);
  }
}

const truncate = (s: string, n = 32) => (s.length > n ? `${s.slice(0, n)}…` : s);
