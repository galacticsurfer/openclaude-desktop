import { useEffect, useMemo, useRef, useState } from 'react';
import { CornerDownLeft, FileText, MessageSquare, Search as SearchIcon, Type } from 'lucide-react';
import * as api from '@/services/api';
import { useConversationStore } from '@/stores/useConversationStore';
import { useUIStore } from '@/stores/useUIStore';
import { renderSnippet } from '@/lib/snippet';
import { formatRelative } from '@/lib/format';
import { Dialog } from '@/components/ui/Dialog';
import { Spinner } from '@/components/ui/Spinner';
import { cn } from '@/lib/cn';
import type { SearchHit } from '@/types';

const ICONS = { title: Type, message: MessageSquare, attachment: FileText, project: FileText };

/**
 * Global search (Ctrl+K).
 *
 * Queries are debounced by 90 ms — long enough to coalesce a burst of
 * keystrokes, short enough that results feel like they appear as you type.
 */
export function SearchPalette() {
  const closeOverlay = useUIStore((s) => s.closeOverlay);
  const open = useConversationStore((s) => s.open);

  const [query, setQuery] = useState('');
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [loading, setLoading] = useState(false);
  const [cursor, setCursor] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const requestSeq = useRef(0);

  useEffect(() => {
    const q = query.trim();
    if (q === '') {
      setHits([]);
      setLoading(false);
      return;
    }

    setLoading(true);
    const seq = ++requestSeq.current;
    const timer = window.setTimeout(async () => {
      try {
        const results = await api.searchAll(q, 60);
        // Ignore a slow earlier query resolving after a newer one.
        if (seq !== requestSeq.current) return;
        setHits(results);
        setCursor(0);
      } catch {
        if (seq === requestSeq.current) setHits([]);
      } finally {
        if (seq === requestSeq.current) setLoading(false);
      }
    }, 90);

    return () => window.clearTimeout(timer);
  }, [query]);

  // Keep the highlighted row in view as the cursor moves.
  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${cursor}"]`)
      ?.scrollIntoView({ block: 'nearest' });
  }, [cursor]);

  const grouped = useMemo(() => {
    const byConversation = new Map<string, SearchHit[]>();
    for (const h of hits) {
      const list = byConversation.get(h.conversationId);
      if (list) list.push(h);
      else byConversation.set(h.conversationId, [h]);
    }
    return [...byConversation.values()].flat();
  }, [hits]);

  function choose(hit: SearchHit) {
    void open(hit.conversationId);
    closeOverlay();
  }

  return (
    <Dialog open onClose={closeOverlay} size="lg" align="top" className="max-h-[70vh]">
      <div className="flex items-center gap-2.5 border-b border-line px-4 py-3">
        <SearchIcon size={16} className="shrink-0 text-ink-faint" aria-hidden />
        <input
          data-autofocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown') {
              e.preventDefault();
              setCursor((c) => Math.min(c + 1, grouped.length - 1));
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              setCursor((c) => Math.max(c - 1, 0));
            } else if (e.key === 'Enter') {
              e.preventDefault();
              const hit = grouped[cursor];
              if (hit) choose(hit);
            }
          }}
          placeholder="Search conversations, messages and file names…"
          aria-label="Search"
          className="flex-1 bg-transparent text-[15px] text-ink outline-none placeholder:text-ink-faint"
        />
        {loading && <Spinner label="Searching" />}
      </div>

      <div ref={listRef} className="max-h-[52vh] overflow-y-auto scroll-thin p-1.5">
        {query.trim() === '' ? (
          <p className="px-3 py-8 text-center text-[13px] text-ink-faint">
            Type to search every conversation on this computer.
          </p>
        ) : grouped.length === 0 && !loading ? (
          <p className="px-3 py-8 text-center text-[13px] text-ink-faint">
            Nothing matched “{query.trim()}”.
          </p>
        ) : (
          grouped.map((hit, i) => {
            const Icon = ICONS[hit.kind] ?? MessageSquare;
            return (
              <button
                key={`${hit.conversationId}-${hit.messageId ?? hit.kind}-${i}`}
                type="button"
                data-index={i}
                onMouseEnter={() => setCursor(i)}
                onClick={() => choose(hit)}
                className={cn(
                  'flex w-full items-start gap-2.5 rounded-md px-3 py-2 text-left transition-colors',
                  cursor === i ? 'bg-sunken' : 'hover:bg-sunken/60',
                )}
              >
                <Icon size={14} className="mt-1 shrink-0 text-ink-faint" aria-hidden />
                <span className="min-w-0 flex-1">
                  <span className="flex items-baseline gap-2">
                    <span className="truncate text-[13.5px] font-medium text-ink">
                      {hit.conversationTitle}
                    </span>
                    <span className="shrink-0 text-[11px] text-ink-faint">
                      {hit.kind === 'message' ? (hit.role === 'user' ? 'you' : 'Claude') : hit.kind}
                      {' · '}
                      {formatRelative(hit.createdAt)}
                    </span>
                  </span>
                  <span className="mt-0.5 line-clamp-2 block text-[12.5px] leading-snug text-ink-soft">
                    {renderSnippet(hit.snippet)}
                  </span>
                </span>
                {cursor === i && (
                  <CornerDownLeft size={13} className="mt-1 shrink-0 text-ink-faint" aria-hidden />
                )}
              </button>
            );
          })
        )}
      </div>

      <footer className="flex items-center gap-4 border-t border-line px-4 py-2 text-[11px] text-ink-faint">
        <span><kbd className="font-mono">↑↓</kbd> navigate</span>
        <span><kbd className="font-mono">↵</kbd> open</span>
        <span><kbd className="font-mono">Esc</kbd> close</span>
        {hits.length > 0 && <span className="ml-auto">{hits.length} results</span>}
      </footer>
    </Dialog>
  );
}
