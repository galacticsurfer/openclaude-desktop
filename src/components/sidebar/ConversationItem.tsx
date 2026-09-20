import { memo } from 'react';
import {
  Archive, ArchiveRestore, Copy, FolderInput, MoreHorizontal, Pencil, Pin, PinOff,
  RotateCcw, Trash2, Download,
} from 'lucide-react';
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
  | 'duplicate' | 'export' | 'move' | 'deleteForever';

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
        onClick={onOpen}
        aria-current={active ? 'page' : undefined}
        className="flex min-w-0 flex-1 items-center gap-2 rounded-md py-1.5 pl-2.5 pr-1 text-left"
      >
        <span className="flex size-3.5 shrink-0 items-center justify-center" aria-hidden>
          {streaming ? (
            <span className="size-1.5 animate-pulse rounded-full bg-accent" />
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

        <span className="shrink-0 pr-1 text-[11px] tabular-nums text-ink-faint opacity-100 transition-opacity group-hover:opacity-0">
          {formatRelative(c.lastMessageAt ?? c.updatedAt)}
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
