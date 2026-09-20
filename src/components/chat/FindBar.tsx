import { useEffect, useRef } from 'react';
import { ChevronDown, ChevronUp, X } from 'lucide-react';
import { IconButton } from '@/components/ui/IconButton';

interface Props {
  query: string;
  onQuery: (v: string) => void;
  total: number;
  active: number;
  onStep: (delta: number) => void;
  onClose: () => void;
}

/** Find within the open conversation. Ctrl+F opens it, Escape closes it. */
export function FindBar({ query, onQuery, total, active, onStep, onClose }: Props) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  return (
    <div className="flex items-center gap-1.5 border-b border-line bg-raised px-3 py-1.5">
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => onQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.preventDefault();
            onClose();
          } else if (e.key === 'Enter') {
            e.preventDefault();
            onStep(e.shiftKey ? -1 : 1);
          }
        }}
        placeholder="Find in conversation…"
        aria-label="Find in conversation"
        className="h-7 flex-1 rounded border border-line bg-surface px-2 text-[13px] text-ink placeholder:text-ink-faint focus:border-line-strong"
      />

      <span
        className="min-w-[4.5rem] text-right text-[12px] tabular-nums text-ink-faint"
        aria-live="polite"
      >
        {query.trim() === '' ? '' : total === 0 ? 'No matches' : `${active + 1} of ${total}`}
      </span>

      <IconButton label="Previous match (Shift Enter)" size="sm" onClick={() => onStep(-1)}>
        <ChevronUp size={14} />
      </IconButton>
      <IconButton label="Next match (Enter)" size="sm" onClick={() => onStep(1)}>
        <ChevronDown size={14} />
      </IconButton>
      <IconButton label="Close find (Esc)" size="sm" onClick={onClose}>
        <X size={14} />
      </IconButton>
    </div>
  );
}
