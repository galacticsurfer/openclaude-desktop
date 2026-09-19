import { useEffect, useMemo, useRef, useState } from 'react';
import {
  Archive, ChevronDown, ChevronRight, FolderPlus, Inbox, MessageSquarePlus,
  Search, Settings as SettingsIcon, Trash2, Folder,
} from 'lucide-react';
import { useConversationStore } from '@/stores/useConversationStore';
import { useUIStore } from '@/stores/useUIStore';
import { dateGroup } from '@/lib/format';
import { cn } from '@/lib/cn';
import { ConversationItem, type ConversationAction } from '@/components/sidebar/ConversationItem';
import { IconButton } from '@/components/ui/IconButton';
import { Empty } from '@/components/ui/Empty';
import { Spinner } from '@/components/ui/Spinner';
import { ConnectionBadge } from '@/components/sidebar/ConnectionBadge';
import { useConversationActions } from '@/hooks/useConversationActions';
import type { ConversationSummary } from '@/types';

export function Sidebar() {
  const {
    conversations, projects, currentId, loadingList, generating,
    open, newConversation, loadConversations, loadProjects,
  } = useConversationStore();
  const { openOverlay, activeProjectId, setActiveProject, scope, setScope } = useUIStore();
  const runAction = useConversationActions();

  const [filter, setFilter] = useState('');
  const [projectsOpen, setProjectsOpen] = useState(true);
  const filterRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void loadConversations();
    void loadProjects();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scope, activeProjectId]);

  // Ctrl+L focuses this field; exposed via a DOM id the shortcut can find.
  useEffect(() => {
    function focusFilter() {
      filterRef.current?.focus();
      filterRef.current?.select();
    }
    document.addEventListener('openclaude:focus-filter', focusFilter);
    return () => document.removeEventListener('openclaude:focus-filter', focusFilter);
  }, []);

  // Client-side narrowing of the already-loaded list. Full-text search across
  // message bodies is Ctrl+K; this is just "find the thread I can see".
  const visible = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return conversations;
    return conversations.filter((c) => c.title.toLowerCase().includes(q));
  }, [conversations, filter]);

  const groups = useMemo(() => groupByDate(visible), [visible]);

  return (
    <aside
      className="flex h-full w-full flex-col border-r border-line bg-surface"
      aria-label="Conversations"
    >
      <div className="flex flex-col gap-2 p-2.5 pb-2">
        <button
          type="button"
          onClick={() => void newConversation()}
          className="flex h-9 w-full items-center gap-2 rounded-md border border-line-strong bg-canvas px-3 text-[13.5px] font-medium text-ink transition-colors hover:border-accent/50 hover:bg-accent-soft"
        >
          <MessageSquarePlus size={15} className="text-accent" aria-hidden />
          New chat
          <kbd className="ml-auto font-mono text-[10.5px] text-ink-faint">Ctrl N</kbd>
        </button>

        <div className="flex items-center gap-1.5">
          <div className="relative min-w-0 flex-1">
            <Search
              size={13}
              aria-hidden
              className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-ink-faint"
            />
            <input
              ref={filterRef}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Escape') {
                  e.stopPropagation();
                  setFilter('');
                  e.currentTarget.blur();
                }
              }}
              placeholder="Filter titles…"
              aria-label="Filter conversations by title"
              className="h-7 w-full rounded border border-transparent bg-sunken pl-7 pr-2 text-[12.5px] text-ink placeholder:text-ink-faint focus:border-line-strong focus:bg-surface"
            />
          </div>
          <IconButton
            label="Search all conversations (Ctrl K)"
            size="sm"
            onClick={() => openOverlay({ kind: 'search' })}
          >
            <Search size={14} />
          </IconButton>
        </div>
      </div>

      {projects.length > 0 && (
        <div className="px-2.5 pb-1">
          <button
            type="button"
            onClick={() => setProjectsOpen((o) => !o)}
            aria-expanded={projectsOpen}
            className="flex w-full items-center gap-1 py-1 text-[11px] font-semibold uppercase tracking-wide text-ink-faint hover:text-ink-soft"
          >
            {projectsOpen ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
            Projects
            <IconButton
              label="New project"
              size="sm"
              className="ml-auto"
              onClick={(e) => {
                e.stopPropagation();
                openOverlay({ kind: 'newProject' });
              }}
            >
              <FolderPlus size={13} />
            </IconButton>
          </button>

          {projectsOpen && (
            <div className="space-y-px pb-1">
              <ProjectRow
                label="All conversations"
                icon={<Inbox size={13} />}
                active={activeProjectId === null}
                onClick={() => setActiveProject(null)}
              />
              {projects.map((p) => (
                <ProjectRow
                  key={p.id}
                  label={p.name}
                  count={p.conversationCount}
                  icon={<Folder size={13} style={p.color ? { color: p.color } : undefined} />}
                  active={activeProjectId === p.id}
                  onClick={() => setActiveProject(p.id)}
                  onEdit={() => openOverlay({ kind: 'editProject', projectId: p.id })}
                />
              ))}
            </div>
          )}
        </div>
      )}

      <nav className="min-h-0 flex-1 overflow-y-auto scroll-thin px-2.5 pb-2" aria-label="Conversation list">
        {loadingList && conversations.length === 0 ? (
          <div className="flex justify-center py-8">
            <Spinner label="Loading conversations" />
          </div>
        ) : visible.length === 0 ? (
          <Empty
            icon={scope === 'trash' ? Trash2 : scope === 'archived' ? Archive : Inbox}
            title={
              filter
                ? 'No matching titles'
                : scope === 'trash'
                  ? 'Trash is empty'
                  : scope === 'archived'
                    ? 'Nothing archived'
                    : 'No conversations yet'
            }
            body={
              filter
                ? 'Press Ctrl+K to search message contents too.'
                : scope === 'active'
                  ? 'Start one with Ctrl+N.'
                  : undefined
            }
          />
        ) : (
          groups.map(([label, items]) => (
            <section key={label} className="mb-3">
              <h2 className="px-2 pb-1 pt-2 text-[11px] font-semibold uppercase tracking-wide text-ink-faint">
                {label}
              </h2>
              <div className="space-y-px">
                {items.map((c) => (
                  <ConversationItem
                    key={c.id}
                    conversation={c}
                    active={c.id === currentId}
                    streaming={generating.includes(c.id)}
                    onOpen={() => void open(c.id)}
                    onAction={(a: ConversationAction) => void runAction(a, c)}
                  />
                ))}
              </div>
            </section>
          ))
        )}
      </nav>

      <footer className="border-t border-line p-2">
        <div className="mb-1.5 flex items-center gap-0.5">
          <ScopeTab label="Active" active={scope === 'active'} onClick={() => setScope('active')} />
          <ScopeTab label="Archived" active={scope === 'archived'} onClick={() => setScope('archived')} />
          <ScopeTab label="Trash" active={scope === 'trash'} onClick={() => setScope('trash')} />
        </div>
        <div className="flex items-center gap-2">
          <ConnectionBadge />
          <IconButton
            label="Settings (Ctrl ,)"
            className="ml-auto"
            onClick={() => openOverlay({ kind: 'settings' })}
          >
            <SettingsIcon size={16} />
          </IconButton>
        </div>
      </footer>
    </aside>
  );
}

function ScopeTab({ label, active, onClick }: { label: string; active: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        'flex-1 rounded px-2 py-1 text-[11.5px] font-medium transition-colors',
        active ? 'bg-sunken text-ink' : 'text-ink-faint hover:text-ink-soft',
      )}
    >
      {label}
    </button>
  );
}

function ProjectRow({
  label, icon, count, active, onClick, onEdit,
}: {
  label: string;
  icon: React.ReactNode;
  count?: number;
  active: boolean;
  onClick: () => void;
  onEdit?: () => void;
}) {
  return (
    <div className={cn('group flex items-center rounded transition-colors', active ? 'bg-sunken' : 'hover:bg-sunken/60')}>
      <button
        type="button"
        onClick={onClick}
        className="flex min-w-0 flex-1 items-center gap-2 px-2 py-1 text-left"
      >
        <span className="shrink-0 text-ink-faint" aria-hidden>{icon}</span>
        <span className={cn('min-w-0 flex-1 truncate text-[13px]', active ? 'text-ink' : 'text-ink-soft')}>
          {label}
        </span>
        {count !== undefined && count > 0 && (
          <span className="shrink-0 text-[11px] tabular-nums text-ink-faint">{count}</span>
        )}
      </button>
      {onEdit && (
        <IconButton
          label={`Edit ${label}`}
          size="sm"
          className="mr-0.5 opacity-0 group-hover:opacity-100"
          onClick={onEdit}
        >
          <SettingsIcon size={12} />
        </IconButton>
      )}
    </div>
  );
}

function groupByDate(items: ConversationSummary[]): Array<[string, ConversationSummary[]]> {
  const pinned = items.filter((c) => c.pinned);
  const rest = items.filter((c) => !c.pinned);

  const buckets = new Map<string, ConversationSummary[]>();
  for (const c of rest) {
    const key = dateGroup(c.lastMessageAt ?? c.updatedAt);
    const list = buckets.get(key);
    if (list) list.push(c);
    else buckets.set(key, [c]);
  }

  const out: Array<[string, ConversationSummary[]]> = [];
  if (pinned.length > 0) out.push(['Pinned', pinned]);
  // Map preserves insertion order, which follows the already-sorted list.
  for (const entry of buckets) out.push(entry);
  return out;
}
