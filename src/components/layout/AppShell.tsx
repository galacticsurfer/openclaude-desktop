import { useEffect, useMemo, useRef } from 'react';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { Sidebar } from './Sidebar';
import { ChatView } from '@/components/chat/ChatView';
import { SearchPalette } from '@/components/search/SearchPalette';
import { CommandPalette } from '@/components/search/CommandPalette';
import { SettingsDialog } from '@/components/settings/SettingsDialog';
import { ProjectDialog } from '@/components/common/ProjectDialog';
import { ConversationInfoDialog } from '@/components/common/ConversationInfoDialog';
import { ConfirmDialog } from '@/components/common/ConfirmDialog';
import { ShortcutsDialog } from '@/components/common/ShortcutsDialog';
import { Toasts } from '@/components/ui/Toasts';
import { useUIStore } from '@/stores/useUIStore';
import { useConversationStore } from '@/stores/useConversationStore';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useHotkeys, type Hotkey } from '@/hooks/useHotkeys';
import { cn } from '@/lib/cn';

export function AppShell() {
  const {
    overlay, openOverlay, closeOverlay, toggleSidebar,
    sidebarCollapsed, setSidebarCollapsed, setDragActive,
  } = useUIStore();
  const newConversation = useConversationStore((s) => s.newConversation);
  const stop = useConversationStore((s) => s.stop);
  const isGenerating = useConversationStore((s) => s.isGenerating);
  const stagePaths = useConversationStore((s) => s.stagePaths);
  const messages = useConversationStore((s) => s.messages);
  const { settings, set } = useSettingsStore();

  // Restore the sidebar state once settings have loaded.
  const restored = useRef(false);
  useEffect(() => {
    if (restored.current || !settings) return;
    restored.current = true;
    setSidebarCollapsed(settings['ui.sidebarCollapsed']);
  }, [settings, setSidebarCollapsed]);

  useEffect(() => {
    if (restored.current && settings && settings['ui.sidebarCollapsed'] !== sidebarCollapsed) {
      void set('ui.sidebarCollapsed', sidebarCollapsed);
    }
  }, [sidebarCollapsed, settings, set]);

  /*
   * Native file drops — the single source of truth for dragging.
   *
   * Tauri intercepts file drops at the webview level (`dragDropEnabled`), and
   * only this event carries real filesystem paths; the DOM `drop` event gets
   * `File` objects with no path, so non-image files cannot be read from it.
   * Handling drops in both places would attach a dropped image twice, so the
   * composer only renders the highlight and takes no part in the drop itself.
   */
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const { payload } = event;
        if (payload.type === 'enter' || payload.type === 'over') {
          setDragActive(true);
        } else if (payload.type === 'leave') {
          setDragActive(false);
        } else if (payload.type === 'drop') {
          setDragActive(false);
          if (payload.paths.length > 0) void stagePaths(payload.paths);
        }
      })
      .then((un) => {
        if (disposed) un();
        else unlisten = un;
      })
      .catch(() => {
        /* drag-drop unavailable; the file picker still works */
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [stagePaths, setDragActive]);

  const overlayOpen = overlay.kind !== 'none';

  const hotkeys = useMemo<Hotkey[]>(
    () => [
      { key: 'n', ctrl: true, allowInInput: true, run: () => void newConversation() },
      { key: 'k', ctrl: true, allowInInput: true, run: () => openOverlay({ kind: 'search' }) },
      { key: 'p', ctrl: true, shift: true, allowInInput: true, run: () => openOverlay({ kind: 'commandPalette' }) },
      { key: ',', ctrl: true, allowInInput: true, run: () => openOverlay({ kind: 'settings' }) },
      { key: '/', ctrl: true, allowInInput: true, run: () => openOverlay({ kind: 'shortcuts' }) },
      { key: 'b', ctrl: true, allowInInput: true, run: toggleSidebar },
      { key: 'n', ctrl: true, shift: true, allowInInput: true, run: () => openOverlay({ kind: 'newProject' }) },
      {
        key: 'l',
        ctrl: true,
        allowInInput: true,
        run: () => document.dispatchEvent(new CustomEvent('openclaude:focus-filter')),
      },
      {
        key: 'c',
        ctrl: true,
        shift: true,
        allowInInput: true,
        run: () => {
          const last = [...messages].reverse().find((m) => m.role === 'assistant' && m.content);
          if (!last) return;
          void import('@tauri-apps/plugin-clipboard-manager').then(({ writeText }) => {
            void writeText(last.content).then(() =>
              useUIStore.getState().toast('info', 'Last reply copied.'),
            );
          });
        },
      },
      {
        key: 'escape',
        allowInInput: true,
        run: () => {
          // Overlays own Escape while they are open; otherwise it stops a
          // running generation.
          if (overlayOpen) closeOverlay();
          else if (isGenerating()) void stop();
        },
      },
    ],
    [newConversation, openOverlay, closeOverlay, toggleSidebar, isGenerating, stop, messages, overlayOpen],
  );

  useHotkeys(hotkeys);

  return (
    <div className="flex h-full overflow-hidden bg-canvas text-ink">
      <a
        href="#main"
        className="sr-only-focusable absolute left-2 top-2 z-[100] rounded bg-accent px-3 py-1.5 text-[13px] text-on-accent"
      >
        Skip to conversation
      </a>

      <div
        // No width transition: animating width relayouts both panes on every
        // frame, which is exactly the jank it was meant to hide. An instant
        // toggle is both snappier and cheaper.
        className={cn(
          'shrink-0 overflow-hidden',
          sidebarCollapsed ? 'w-0' : 'w-[272px]',
        )}
      >
        <Sidebar />
      </div>

      <main id="main" className="min-w-0 flex-1">
        <ChatView />
      </main>

      {overlay.kind === 'search' && <SearchPalette />}
      {overlay.kind === 'commandPalette' && <CommandPalette />}
      {overlay.kind === 'settings' && <SettingsDialog section={overlay.section} />}
      {overlay.kind === 'shortcuts' && <ShortcutsDialog />}
      {overlay.kind === 'newProject' && <ProjectDialog />}
      {overlay.kind === 'editProject' && <ProjectDialog projectId={overlay.projectId} />}
      {overlay.kind === 'conversationInfo' && (
        <ConversationInfoDialog conversationId={overlay.conversationId} />
      )}
      {overlay.kind === 'confirm' && <ConfirmDialog request={overlay.request} />}

      <Toasts />
    </div>
  );
}
