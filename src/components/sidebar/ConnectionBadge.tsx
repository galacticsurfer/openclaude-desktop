import { useEffect, useState } from 'react';
import { KeyRound, ShieldAlert, Wifi, WifiOff } from 'lucide-react';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { cn } from '@/lib/cn';

/**
 * Connection and credential state.
 *
 * Offline is detected with `navigator.onLine`, which in WebKitGTK reflects
 * NetworkManager's view — good enough to explain a failure, and every send
 * still surfaces the real error if it is wrong.
 */
export function ConnectionBadge() {
  const credentials = useSettingsStore((s) => s.credentials);
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

  const oauth = credentials?.mode === 'oauth';

  const state = !credentials?.configured
    ? oauth
      ? {
          icon: KeyRound,
          tone: 'text-warn',
          label: 'Not signed in',
          hint: 'Sign in with your browser, or add an API key',
        }
      : { icon: KeyRound, tone: 'text-warn', label: 'No API key', hint: 'Add your Anthropic API key' }
    : !online
      ? { icon: WifiOff, tone: 'text-ink-faint', label: 'Offline', hint: 'History is still available' }
      : oauth
        ? {
            icon: Wifi,
            tone: 'text-success',
            label: 'Signed in',
            hint: 'Using browser sign-in via the Anthropic CLI',
          }
        : credentials.backend === 'memoryOnly'
          ? {
              icon: ShieldAlert,
              tone: 'text-warn',
              label: 'Key not saved',
              hint: 'No system keyring — the key is held for this session only',
            }
          : { icon: Wifi, tone: 'text-success', label: 'Connected', hint: 'Ready' };

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
    </button>
  );
}
