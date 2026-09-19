import { AlertCircle, CheckCircle2, Info, X } from 'lucide-react';
import { useUIStore } from '@/stores/useUIStore';
import { cn } from '@/lib/cn';

const ICONS = {
  info: Info,
  error: AlertCircle,
  success: CheckCircle2,
} as const;

const TONE = {
  info: 'text-ink-soft',
  error: 'text-danger',
  success: 'text-success',
} as const;

export function Toasts() {
  const toasts = useUIStore((s) => s.toasts);
  const dismiss = useUIStore((s) => s.dismissToast);

  if (toasts.length === 0) return null;

  return (
    <div
      className="pointer-events-none fixed bottom-4 left-1/2 z-[60] flex w-full max-w-md -translate-x-1/2 flex-col gap-2 px-4"
      role="region"
      aria-label="Notifications"
    >
      {toasts.map((t) => {
        const Icon = ICONS[t.kind];
        return (
          <div
            key={t.id}
            role={t.kind === 'error' ? 'alert' : 'status'}
            className="pointer-events-auto flex items-start gap-2.5 rounded-lg border border-line bg-raised px-3.5 py-2.5 shadow-raised animate-slide-up"
          >
            <Icon size={16} className={cn('mt-0.5 shrink-0', TONE[t.kind])} aria-hidden />
            <p className="min-w-0 flex-1 text-[13px] leading-snug text-ink">{t.message}</p>
            {t.action && (
              <button
                type="button"
                onClick={() => {
                  t.action?.run();
                  dismiss(t.id);
                }}
                className="shrink-0 text-[13px] font-medium text-accent hover:underline"
              >
                {t.action.label}
              </button>
            )}
            <button
              type="button"
              aria-label="Dismiss"
              onClick={() => dismiss(t.id)}
              className="-mr-1 shrink-0 rounded p-0.5 text-ink-faint hover:text-ink"
            >
              <X size={14} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
