import { useEffect, useRef, useState, type ReactNode } from 'react';
import { cn } from '@/lib/cn';

export interface MenuItem {
  label: string;
  icon?: ReactNode;
  onSelect: () => void;
  danger?: boolean;
  disabled?: boolean;
  /** Draws a divider above this item. */
  separated?: boolean;
  shortcut?: string;
}

interface Props {
  items: MenuItem[];
  trigger: (props: { onClick: (e: React.MouseEvent) => void; 'aria-expanded': boolean }) => ReactNode;
  align?: 'left' | 'right';
}

/**
 * A small dropdown menu with roving keyboard focus.
 *
 * Positioned with `fixed` from the trigger's measured rect so it is never
 * clipped by the sidebar's `overflow: hidden`.
 */
export function Menu({ items, trigger, align = 'right' }: Props) {
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);
  const [cursor, setCursor] = useState(0);
  const menuRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;

    function onPointerDown(e: PointerEvent) {
      if (!menuRef.current?.contains(e.target as Node)) setOpen(false);
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.stopPropagation();
        setOpen(false);
        triggerRef.current?.focus?.();
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        setCursor((c) => next(items, c, 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setCursor((c) => next(items, c, -1));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        const item = items[cursor];
        if (item && !item.disabled) {
          setOpen(false);
          item.onSelect();
        }
      }
    }
    // Any scroll would leave the fixed menu stranded.
    function onScroll() {
      setOpen(false);
    }

    document.addEventListener('pointerdown', onPointerDown, true);
    document.addEventListener('keydown', onKey, true);
    window.addEventListener('scroll', onScroll, true);
    window.addEventListener('resize', onScroll);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown, true);
      document.removeEventListener('keydown', onKey, true);
      window.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', onScroll);
    };
  }, [open, items, cursor]);

  return (
    <>
      {trigger({
        'aria-expanded': open,
        onClick: (e) => {
          e.preventDefault();
          e.stopPropagation();
          const el = e.currentTarget as HTMLElement;
          triggerRef.current = el;
          const r = el.getBoundingClientRect();
          const width = 216;
          setPos({
            top: Math.min(r.bottom + 4, window.innerHeight - 8),
            left: align === 'right' ? Math.max(8, r.right - width) : r.left,
          });
          setCursor(items.findIndex((i) => !i.disabled));
          setOpen((o) => !o);
        },
      })}

      {open && pos && (
        <div
          ref={menuRef}
          role="menu"
          style={{ top: pos.top, left: pos.left, width: 216 }}
          className="fixed z-[70] overflow-hidden rounded-lg border border-line bg-raised py-1 shadow-overlay animate-fade-in"
        >
          {items.map((item, i) => (
            <div key={item.label}>
              {item.separated && <div className="my-1 h-px bg-line" role="separator" />}
              <button
                type="button"
                role="menuitem"
                disabled={item.disabled}
                onMouseEnter={() => setCursor(i)}
                onClick={() => {
                  setOpen(false);
                  item.onSelect();
                }}
                className={cn(
                  'flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-[13px] transition-colors',
                  'disabled:cursor-not-allowed disabled:opacity-40',
                  item.danger ? 'text-danger' : 'text-ink',
                  cursor === i && !item.disabled && (item.danger ? 'bg-danger-soft' : 'bg-sunken'),
                )}
              >
                {item.icon && (
                  <span className="shrink-0 text-ink-faint [button:disabled_&]:opacity-50">
                    {item.icon}
                  </span>
                )}
                <span className="min-w-0 flex-1 truncate">{item.label}</span>
                {item.shortcut && (
                  <kbd className="shrink-0 font-mono text-[11px] text-ink-faint">
                    {item.shortcut}
                  </kbd>
                )}
              </button>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

function next(items: MenuItem[], from: number, dir: 1 | -1): number {
  for (let step = 1; step <= items.length; step++) {
    const i = (from + dir * step + items.length * 2) % items.length;
    if (!items[i]?.disabled) return i;
  }
  return from;
}
