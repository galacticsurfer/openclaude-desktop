import { useMemo, useState } from 'react';
import {
  Archive, Download, FolderPlus, Keyboard, MessageSquarePlus, Moon, PanelLeft,
  RefreshCw, Search, Settings, Sun, Trash2, GitBranch, Monitor,
} from 'lucide-react';
import { useConversationStore } from '@/stores/useConversationStore';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { exportConversation } from '@/hooks/useConversationActions';
import { Dialog } from '@/components/ui/Dialog';
import { cn } from '@/lib/cn';
import * as api from '@/services/api';
import type { ThemePreference } from '@/types';

interface Command {
  id: string;
  label: string;
  hint?: string;
  icon: React.ReactNode;
  shortcut?: string;
  run: () => void;
  disabled?: boolean;
}

export function CommandPalette() {
  const { closeOverlay, openOverlay, toggleSidebar, toast, setScope } = useUIStore();
  const { newConversation, current, loadConversations, open } = useConversationStore();
  const { settings, set, refreshModels } = useSettingsStore();
  const [query, setQuery] = useState('');
  const [cursor, setCursor] = useState(0);

  const commands = useMemo<Command[]>(() => {
    const theme = settings?.['appearance.theme'] ?? 'system';
    const cycle: Record<ThemePreference, ThemePreference> = {
      system: 'light',
      light: 'dark',
      dark: 'system',
    };
    const themeIcon = theme === 'dark' ? Moon : theme === 'light' ? Sun : Monitor;
    const ThemeIcon = themeIcon;

    const list: Command[] = [
      {
        id: 'new',
        label: 'New conversation',
        icon: <MessageSquarePlus size={15} />,
        shortcut: 'Ctrl N',
        run: () => void newConversation(),
      },
      {
        id: 'search',
        label: 'Search conversations',
        icon: <Search size={15} />,
        shortcut: 'Ctrl K',
        run: () => openOverlay({ kind: 'search' }),
      },
      {
        id: 'new-project',
        label: 'New project',
        icon: <FolderPlus size={15} />,
        shortcut: 'Ctrl Shift N',
        run: () => openOverlay({ kind: 'newProject' }),
      },
      {
        id: 'theme',
        label: `Theme: ${theme} — switch to ${cycle[theme]}`,
        icon: <ThemeIcon size={15} />,
        run: () => void set('appearance.theme', cycle[theme]),
      },
      {
        id: 'sidebar',
        label: 'Toggle sidebar',
        icon: <PanelLeft size={15} />,
        shortcut: 'Ctrl B',
        run: toggleSidebar,
      },
      {
        id: 'settings',
        label: 'Settings',
        icon: <Settings size={15} />,
        shortcut: 'Ctrl ,',
        run: () => openOverlay({ kind: 'settings' }),
      },
      {
        id: 'shortcuts',
        label: 'Keyboard shortcuts',
        icon: <Keyboard size={15} />,
        shortcut: 'Ctrl /',
        run: () => openOverlay({ kind: 'shortcuts' }),
      },
      {
        id: 'models',
        label: 'Refresh model list',
        icon: <RefreshCw size={15} />,
        run: () => {
          void refreshModels(true).then(() => toast('success', 'Model list refreshed.'));
        },
      },
      {
        id: 'mcp',
        label: 'Manage MCP servers',
        hint: 'Coming in a later release',
        icon: <GitBranch size={15} />,
        disabled: true,
        run: () => {},
      },
      {
        id: 'trash-view',
        label: 'Show trash',
        icon: <Trash2 size={15} />,
        run: () => setScope('trash'),
      },
      {
        id: 'archived-view',
        label: 'Show archived',
        icon: <Archive size={15} />,
        run: () => setScope('archived'),
      },
    ];

    if (current) {
      list.splice(
        3,
        0,
        {
          id: 'export-md',
          label: 'Export this conversation as Markdown',
          icon: <Download size={15} />,
          run: () => void exportConversation(current.id, 'markdown', toast),
        },
        {
          id: 'export-json',
          label: 'Export this conversation as JSON',
          icon: <Download size={15} />,
          run: () => void exportConversation(current.id, 'json', toast),
        },
        {
          id: 'archive-current',
          label: current.archived ? 'Unarchive this conversation' : 'Archive this conversation',
          icon: <Archive size={15} />,
          run: () => {
            void api
              .setConversationArchived(current.id, !current.archived)
              .then(() => loadConversations())
              .then(() => open(current.id));
          },
        },
      );
    }

    return list;
  }, [
    settings, current, newConversation, openOverlay, toggleSidebar, set,
    refreshModels, toast, setScope, loadConversations, open,
  ]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands;
    return commands.filter((c) => c.label.toLowerCase().includes(q));
  }, [commands, query]);

  function runAt(index: number) {
    const cmd = filtered[index];
    if (!cmd || cmd.disabled) return;
    closeOverlay();
    cmd.run();
  }

  return (
    <Dialog open onClose={closeOverlay} size="md" align="top">
      <div className="border-b border-line px-4 py-3">
        <input
          data-autofocus
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setCursor(0);
          }}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown') {
              e.preventDefault();
              setCursor((c) => Math.min(c + 1, filtered.length - 1));
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              setCursor((c) => Math.max(c - 1, 0));
            } else if (e.key === 'Enter') {
              e.preventDefault();
              runAt(cursor);
            }
          }}
          placeholder="Type a command…"
          aria-label="Command"
          className="w-full bg-transparent text-[15px] text-ink outline-none placeholder:text-ink-faint"
        />
      </div>

      <div className="max-h-[50vh] overflow-y-auto scroll-thin p-1.5">
        {filtered.length === 0 ? (
          <p className="px-3 py-6 text-center text-[13px] text-ink-faint">No matching command.</p>
        ) : (
          filtered.map((c, i) => (
            <button
              key={c.id}
              type="button"
              disabled={c.disabled}
              onMouseEnter={() => setCursor(i)}
              onClick={() => runAt(i)}
              className={cn(
                'flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-left text-[13.5px] transition-colors',
                'disabled:cursor-not-allowed disabled:opacity-40',
                cursor === i && !c.disabled ? 'bg-sunken' : 'hover:bg-sunken/60',
              )}
            >
              <span className="shrink-0 text-ink-faint" aria-hidden>{c.icon}</span>
              <span className="min-w-0 flex-1 truncate text-ink">{c.label}</span>
              {c.hint && <span className="shrink-0 text-[11px] text-ink-faint">{c.hint}</span>}
              {c.shortcut && (
                <kbd className="shrink-0 font-mono text-[11px] text-ink-faint">{c.shortcut}</kbd>
              )}
            </button>
          ))
        )}
      </div>
    </Dialog>
  );
}
