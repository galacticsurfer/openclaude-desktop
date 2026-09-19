import { useEffect, useRef, type ReactNode } from 'react';
import { X } from 'lucide-react';
import { cn } from '@/lib/cn';
import { IconButton } from './IconButton';

interface Props {
  open: boolean;
  onClose: () => void;
  title?: string;
  description?: string;
  children: ReactNode;
  footer?: ReactNode;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  /** Search-style overlays sit high on screen rather than centred. */
  align?: 'center' | 'top';
  className?: string;
}

const SIZES = { sm: 'max-w-sm', md: 'max-w-lg', lg: 'max-w-2xl', xl: 'max-w-4xl' };

/**
 * Modal dialog with a focus trap.
 *
 * Implemented directly rather than pulled from a component library: the app
 * needs exactly one modal pattern, and this keeps the dependency list short
 * while still doing the accessibility work properly — Escape to close, focus
 * moved in on open and restored on close, and Tab cycling within.
 */
export function Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  size = 'md',
  align = 'center',
  className,
}: Props) {
  const panelRef = useRef<HTMLDivElement>(null);
  const restoreTo = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;

    restoreTo.current = document.activeElement as HTMLElement | null;

    // Focus the first sensible control, or the panel itself.
    const timer = window.setTimeout(() => {
      const panel = panelRef.current;
      if (!panel) return;
      const target = panel.querySelector<HTMLElement>(
        '[data-autofocus], input:not([type="hidden"]), textarea, select, button',
      );
      (target ?? panel).focus();
    }, 0);

    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
        return;
      }
      if (e.key !== 'Tab') return;

      const panel = panelRef.current;
      if (!panel) return;
      const focusable = [
        ...panel.querySelectorAll<HTMLElement>(
          'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
        ),
      ].filter((el) => el.offsetParent !== null || el === document.activeElement);

      if (focusable.length === 0) return;
      const first = focusable[0]!;
      const last = focusable[focusable.length - 1]!;

      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }

    document.addEventListener('keydown', onKeyDown, true);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener('keydown', onKeyDown, true);
      restoreTo.current?.focus?.();
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className={cn(
        'fixed inset-0 z-50 flex justify-center bg-black/35 p-4 animate-fade-in',
        align === 'top' ? 'items-start pt-[12vh]' : 'items-center',
      )}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        className={cn(
          'flex max-h-[85vh] w-full flex-col overflow-hidden rounded-xl border border-line',
          'bg-raised shadow-overlay outline-none animate-slide-up',
          SIZES[size],
          className,
        )}
      >
        {title && (
          <header className="flex items-start justify-between gap-4 border-b border-line px-5 py-3.5">
            <div className="min-w-0">
              <h2 className="text-[15px] font-semibold tracking-[-0.01em] text-ink">{title}</h2>
              {description && (
                <p className="mt-1 text-[13px] leading-snug text-ink-soft">{description}</p>
              )}
            </div>
            <IconButton label="Close" onClick={onClose} size="sm" className="-mr-1 mt-0.5">
              <X size={15} />
            </IconButton>
          </header>
        )}

        <div className="min-h-0 flex-1 overflow-y-auto scroll-thin">{children}</div>

        {footer && (
          <footer className="flex items-center justify-end gap-2 border-t border-line bg-sunken/50 px-5 py-3">
            {footer}
          </footer>
        )}
      </div>
    </div>
  );
}
