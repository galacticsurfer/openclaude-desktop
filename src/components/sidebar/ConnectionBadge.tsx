import { useEffect, useState } from 'react';
import { Terminal, TriangleAlert, Wifi, WifiOff } from 'lucide-react';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { cn } from '@/lib/cn';

/**
 * Backend and connectivity state.
 *
 * Offline comes from `navigator.onLine`, which in WebKitGTK reflects
 * NetworkManager — good enough to explain a failure, and every send still
 * surfaces the real error if it is wrong.
 */
export function ConnectionBadge() {
  const claudeCode = useSettingsStore((s) => s.claudeCode);
  const openOverlay = useUIStore((s) => s.openOverlay);
  const [online, setOnline] = useState(() => navigator.onLine);

  useEffect(() => {
    const up = () => setOnline(true);
    const down = () => setOnline(false);
    window.addEventListener('online', up);
    window.addEventListener('offline', down);
    return () => {
      window.removeEventListener('online', up);
      window.removeEventListener('offline', down);
    };
  }, []);

  const state = !claudeCode?.installed
    ? {
        icon: TriangleAlert,
        tone: 'text-warn',
        label: 'Claude Code missing',
        hint: 'Install the claude command to send messages',
      }
    : !online
      ? {
          icon: WifiOff,
          tone: 'text-ink-faint',
          label: 'Offline',
          hint: 'History is still available',
        }
      : {
          icon: Wifi,
          tone: 'text-success',
          label: 'Claude Code',
          hint: claudeCode.version ?? 'Connected through Claude Code',
        };

  const Icon = state.icon;

  return (
    <button
      type="button"
      onClick={() => openOverlay({ kind: 'settings', section: 'claude' })}
      title={state.hint}
      className="flex min-w-0 items-center gap-1.5 rounded px-1.5 py-1 text-left transition-colors hover:bg-sunken"
    >
      <Icon size={13} className={cn('shrink-0', state.tone)} aria-hidden />
      <span className="truncate text-[11.5px] text-ink-faint">{state.label}</span>
      <Terminal size={10} className="shrink-0 text-ink-faint/60" aria-hidden />
    </button>
  );
}
