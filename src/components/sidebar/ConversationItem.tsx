import { memo } from 'react';
import {
  Archive, ArchiveRestore, Copy, FolderInput, MoreHorizontal, Pencil, Pin, PinOff,
  RotateCcw, Trash2, Download, Columns2 } from 'lucide-react';
import type { ConversationSummary } from '@/types';
import { formatRelative } from '@/lib/format';
import { cn } from '@/lib/cn';
import { Menu, type MenuItem } from '@/components/ui/Menu';

interface Props {
  conversation: ConversationSummary;
  active: boolean;
  streaming: boolean;
  onOpen: () => void;
  onAction: (action: ConversationAction) => void;
}

export type ConversationAction =
  | 'rename' | 'pin' | 'archive' | 'trash' | 'restore'
  | 'duplicate' | 'export' | 'move' | 'deleteForever' | 'newTab';

export const ConversationItem = memo(function ConversationItem({
  conversation: c,
  active,
  streaming,
  onOpen,
  onAction,
}: Props) {
  const inTrash = c.deletedAt !== null;

  const items: MenuItem[] = inTrash
    ? [
        { label: 'Restore', icon: <RotateCcw size={14} />, onSelect: () => onAction('restore') },
        {
          label: 'Delete permanently',
          icon: <Trash2 size={14} />,
          danger: true,
          separated: true,
          onSelect: () => onAction('deleteForever'),
        },
      ]
    : [
        {
          label: 'Open in new tab',
          icon: <Columns2 size={14} />,
          onSelect: () => onAction('newTab'),
        },
        { label: 'Rename', icon: <Pencil size={14} />, onSelect: () => onAction('rename') },
        {
          label: c.pinned ? 'Unpin' : 'Pin to top',
          icon: c.pinned ? <PinOff size={14} /> : <Pin size={14} />,
          onSelect: () => onAction('pin'),
        },
        { label: 'Move to project…', icon: <FolderInput size={14} />, onSelect: () => onAction('move') },
        { label: 'Duplicate', icon: <Copy size={14} />, separated: true, onSelect: () => onAction('duplicate') },
        { label: 'Export…', icon: <Download size={14} />, onSelect: () => onAction('export') },
        {
          label: c.archived ? 'Unarchive' : 'Archive',
          icon: c.archived ? <ArchiveRestore size={14} /> : <Archive size={14} />,
          separated: true,
          onSelect: () => onAction('archive'),
        },
        { label: 'Move to trash', icon: <Trash2 size={14} />, danger: true, onSelect: () => onAction('trash') },
      ];

  return (
    <div
      className={cn(
        'conversation-row group relative flex items-center rounded-md transition-colors',
        active ? 'bg-accent-soft' : 'hover:bg-sunken',
      )}
    >
      <button
        type="button"
        onClick={(e) => (e.ctrlKey || e.metaKey ? onAction('newTab') : onOpen())}
        onAuxClick={(e) => {
          if (e.button === 1) {
            e.preventDefault();
            onAction('newTab');
          }
        }}
        aria-current={active ? 'page' : undefined}
        aria-describedby={streaming ? `${c.id}-working` : undefined}
        className="flex min-w-0 flex-1 items-center gap-2 rounded-md py-1.5 pl-2.5 pr-1 text-left"
      >
        <span className="flex size-3.5 shrink-0 items-center justify-center" aria-hidden>
          {streaming ? (
            // A pulsing dot read as the same thing as the active-row dot, so
            // a reply in progress was invisible. A ring that actually spins
            // is unambiguous, and distinguishes *working* from *selected*.
            <span className="size-3 animate-spin rounded-full border-[1.5px] border-accent border-r-transparent" />
          ) : c.pinned ? (
            <Pin size={11} className="text-ink-faint" fill="currentColor" />
          ) : (
            <span
              className={cn(
                'size-1.5 rounded-full',
                active ? 'bg-accent' : 'bg-line-strong group-hover:bg-ink-faint',
              )}
            />
          )}
        </span>

        {streaming && (
          <span id={`${c.id}-working`} className="sr-only">
            Claude is replying
          </span>
        )}

        <span className="min-w-0 flex-1">
          <span
            className={cn(
              'block truncate text-[13.5px] leading-snug',
              active ? 'font-medium text-ink' : 'text-ink-soft group-hover:text-ink',
            )}
          >
            {c.title}
          </span>
        </span>

        <span
          className={cn(
            'shrink-0 pr-1 text-[11px] tabular-nums transition-opacity group-hover:opacity-0',
            streaming ? 'text-accent' : 'text-ink-faint',
          )}
        >
          {streaming ? 'working…' : formatRelative(c.lastMessageAt ?? c.updatedAt)}
        </span>
      </button>

      <div className="absolute right-1 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
        <Menu
          items={items}
          trigger={(props) => (
            <button
              type="button"
              aria-label={`Actions for ${c.title}`}
              className="flex size-6 items-center justify-center rounded text-ink-faint hover:bg-line hover:text-ink"
              {...props}
            >
              <MoreHorizontal size={14} />
            </button>
          )}
        />
      </div>
    </div>
  );
});
