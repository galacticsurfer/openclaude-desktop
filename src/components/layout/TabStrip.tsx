import { X } from 'lucide-react';
import { useConversationStore } from '@/stores/useConversationStore';
import { useUIStore } from '@/stores/useUIStore';
import { cn } from '@/lib/cn';

/**
 * Open conversations, as tabs.
 *
 * Hidden until there are at least two: a strip showing a single tab is
 * chrome that tells the user nothing they cannot already see in the header.
 */
export function TabStrip() {
  const tabs = useUIStore((s) => s.tabs);
  const closeTab = useUIStore((s) => s.closeTab);
  const conversations = useConversationStore((s) => s.conversations);
  const currentId = useConversationStore((s) => s.currentId);
  const open = useConversationStore((s) => s.open);
  const clearCurrent = useConversationStore((s) => s.clearCurrent);

  if (tabs.length < 2) return null;

  const titleOf = (id: string) =>
    conversations.find((c) => c.id === id)?.title ?? 'Conversation';

  return (
    <div
      role="tablist"
      aria-label="Open conversations"
      className="flex shrink-0 items-stretch gap-0.5 overflow-x-auto scroll-thin border-b border-line bg-sunken px-1.5 pt-1"
    >
      {tabs.map((id) => {
        const active = id === currentId;
        return (
          <div
            key={id}
            className={cn(
              'group/tab flex min-w-0 max-w-[13rem] items-center gap-1 rounded-t-md border border-b-0 px-2 py-1',
              active
                ? 'border-line bg-canvas'
                : 'border-transparent text-ink-faint hover:bg-surface',
            )}
          >
            <button
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => void open(id)}
              // Middle click closes, as everywhere else with tabs.
              onAuxClick={(e) => {
                if (e.button === 1) {
                  e.preventDefault();
                  const next = closeTab(id, currentId);
                  if (next) void open(next);
                  else clearCurrent();
                }
              }}
              className={cn(
                'min-w-0 flex-1 truncate text-left text-[12.5px]',
                active ? 'text-ink' : 'text-ink-soft',
              )}
            >
              {titleOf(id)}
            </button>
            <button
              type="button"
              aria-label={`Close ${titleOf(id)}`}
              onClick={() => {
                const next = closeTab(id, currentId);
                if (next) void open(next);
                else clearCurrent();
              }}
              className="shrink-0 rounded p-0.5 text-ink-faint opacity-0 hover:bg-line hover:text-ink focus-visible:opacity-100 group-hover/tab:opacity-100"
            >
              <X size={12} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
