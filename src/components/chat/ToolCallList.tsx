import { useState } from 'react';
import { AlertTriangle, Check, ChevronDown, ChevronRight, Wrench } from 'lucide-react';
import type { ToolCallRecord } from '@/types';

/** Trim a long argument value so one field cannot swamp the transcript. */
function preview(value: unknown): string {
  const text = typeof value === 'string' ? value : JSON.stringify(value);
  if (text === undefined) return '';
  return text.length > 300 ? `${text.slice(0, 300)}…` : text;
}

/**
 * What Claude ran, and what it ran it with.
 *
 * The arguments are the point. Knowing `mcp__notes__search` was called says
 * very little; knowing it searched for "quarterly report" is what makes a
 * granted tool auditable after the fact. They are collapsed by default,
 * because most calls are unremarkable and a wall of JSON in the middle of a
 * reply is its own kind of unreadable.
 */
export function ToolCallList({ calls }: { calls: ToolCallRecord[] }) {
  const [open, setOpen] = useState<string | null>(null);

  return (
    <ul className="mb-2 space-y-1">
      {calls.map((t) => {
        const args = t.input as Record<string, unknown> | undefined;
        const fields = args && typeof args === 'object' ? Object.entries(args) : [];
        const expanded = open === t.id;

        return (
          <li key={t.id} className="overflow-hidden rounded-md border border-line bg-sunken/60">
            <button
              type="button"
              disabled={fields.length === 0}
              onClick={() => setOpen((o) => (o === t.id ? null : t.id))}
              aria-expanded={fields.length === 0 ? undefined : expanded}
              className="flex w-full items-center gap-1.5 px-2.5 py-1 text-left text-[12px] disabled:cursor-default"
            >
              {fields.length > 0 ? (
                expanded ? (
                  <ChevronDown size={12} className="shrink-0 text-ink-faint" />
                ) : (
                  <ChevronRight size={12} className="shrink-0 text-ink-faint" />
                )
              ) : (
                <Wrench size={12} className="shrink-0 text-ink-faint" aria-hidden />
              )}
              <code className="min-w-0 flex-1 truncate font-mono text-ink-soft">{t.name}</code>
              {t.ok === undefined ? (
                <span className="text-ink-faint">running…</span>
              ) : t.ok ? (
                <Check size={12} className="text-success" aria-label="succeeded" />
              ) : (
                <AlertTriangle size={12} className="text-danger" aria-label="failed" />
              )}
            </button>

            {expanded && fields.length > 0 && (
              <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 border-t border-line px-2.5 py-1.5 font-mono text-[11.5px]">
                {fields.map(([k, v]) => (
                  <div key={k} className="contents">
                    <dt className="text-ink-faint">{k}</dt>
                    <dd className="min-w-0 whitespace-pre-wrap break-words text-ink-soft">
                      {preview(v)}
                    </dd>
                  </div>
                ))}
              </dl>
            )}
          </li>
        );
      })}
    </ul>
  );
}
