import { useEffect, useMemo } from 'react';
import { CornerDownLeft, Slash } from 'lucide-react';
import { cn } from '@/lib/cn';

interface Props {
  /** Text currently in the composer. */
  value: string;
  commands: string[];
  cursor: number;
  onCursorChange: (i: number) => void;
  onPick: (command: string) => void;
}

/**
 * The slash commands Claude Code offers, completed inline.
 *
 * The list is not configured anywhere — it is whatever the CLI reported for
 * this installation (plugins and skills included), captured from the session
 * it starts and cached. Terminal-only commands are filtered out upstream,
 * since they would do nothing through a pipe.
 */
export function slashQuery(value: string): string | null {
  // Only a leading slash opens the menu; a slash mid-sentence is just text.
  if (!value.startsWith('/')) return null;
  const rest = value.slice(1);
  // Once there is an argument the command is chosen; stop suggesting.
  if (/\s/.test(rest)) return null;
  return rest;
}

export function matchCommands(commands: string[], query: string): string[] {
  const q = query.toLowerCase();
  if (!q) return commands.slice(0, 50);
  const starts = commands.filter((c) => c.toLowerCase().startsWith(q));
  const contains = commands.filter(
    (c) => !c.toLowerCase().startsWith(q) && c.toLowerCase().includes(q),
  );
  return [...starts, ...contains].slice(0, 50);
}

export function SlashCommandMenu({ value, commands, cursor, onCursorChange, onPick }: Props) {
  const query = slashQuery(value);
  const matches = useMemo(
    () => (query === null ? [] : matchCommands(commands, query)),
    [commands, query],
  );

  // Keep the highlighted row in range as the query narrows.
  useEffect(() => {
    if (cursor > matches.length - 1) onCursorChange(0);
  }, [matches.length, cursor, onCursorChange]);

  if (query === null || matches.length === 0) return null;

  return (
    <div
      role="listbox"
      aria-label="Slash commands"
      className="absolute bottom-full left-0 right-0 z-20 mb-2 max-h-72 overflow-y-auto scroll-thin rounded-lg border border-line bg-raised py-1 shadow-overlay animate-slide-up"
    >
      {matches.map((c, i) => (
        <button
          key={c}
          type="button"
          role="option"
          aria-selected={i === cursor}
          onMouseEnter={() => onCursorChange(i)}
          onMouseDown={(e) => {
            // mousedown, not click: the textarea must not lose focus first.
            e.preventDefault();
            onPick(c);
          }}
          className={cn(
            'flex w-full items-center gap-2 px-3 py-1.5 text-left text-[13px] transition-colors',
            i === cursor ? 'bg-sunken' : 'hover:bg-sunken/60',
          )}
        >
          <Slash size={11} className="shrink-0 text-ink-faint" aria-hidden />
          <span className="min-w-0 flex-1 truncate font-mono text-ink">{c}</span>
          {i === cursor && (
            <CornerDownLeft size={12} className="shrink-0 text-ink-faint" aria-hidden />
          )}
        </button>
      ))}
      <p className="border-t border-line px-3 pb-0.5 pt-1.5 text-[11px] text-ink-faint">
        {matches.length} command{matches.length === 1 ? '' : 's'} from Claude Code · Tab to
        complete
      </p>
    </div>
  );
}
