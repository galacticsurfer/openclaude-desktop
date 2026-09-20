import { useEffect, useState } from 'react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { Button } from '@/components/ui/Button';

/**
 * Record a desktop-wide shortcut by pressing it.
 *
 * Typing an accelerator by hand invites typos in a string the user cannot
 * test without restarting, so the field captures the real key combination
 * instead. The backend claims it immediately and reports a clash, because a
 * shortcut another application already owns would otherwise just appear to
 * do nothing.
 */
export function QuickChatShortcut() {
  const settings = useSettingsStore((s) => s.settings);
  const reload = useSettingsStore((s) => s.load);
  const toast = useUIStore((s) => s.toast);

  const saved = (settings?.['general.quickChatShortcut'] as string | undefined) ?? '';
  const [capturing, setCapturing] = useState(false);
  const [pending, setPending] = useState<string | null>(null);

  useEffect(() => {
    if (!capturing) return;

    function onKey(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === 'Escape') {
        setCapturing(false);
        setPending(null);
        return;
      }
      // Wait for a non-modifier: Ctrl alone is not a shortcut.
      if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;

      const parts: string[] = [];
      if (e.ctrlKey) parts.push('Ctrl');
      if (e.altKey) parts.push('Alt');
      if (e.shiftKey) parts.push('Shift');
      if (e.metaKey) parts.push('Super');
      // A bare letter would be claimed from the whole desktop.
      if (parts.length === 0) {
        toast('error', 'A global shortcut needs at least one modifier.');
        return;
      }
      parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);
      setPending(parts.join('+'));
      setCapturing(false);
    }

    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [capturing, toast]);

  async function apply(accelerator: string) {
    try {
      await api.setQuickChatShortcut(accelerator);
      await reload();
      setPending(null);
      toast('success', accelerator === '' ? 'Shortcut cleared.' : `${accelerator} is ready.`);
    } catch (err) {
      toast('error', AppError.from(err).message);
    }
  }

  const shown = pending ?? saved;

  return (
    <div className="flex flex-col items-stretch gap-1.5">
      <button
        type="button"
        onClick={() => setCapturing(true)}
        className="h-8 rounded border border-line bg-surface px-2 font-mono text-[12.5px] text-ink hover:border-line-strong"
      >
        {capturing ? 'Press keys…' : shown === '' ? 'Not set' : shown}
      </button>
      <div className="flex gap-1.5">
        {pending !== null && (
          <Button size="sm" onClick={() => void apply(pending)}>
            Save
          </Button>
        )}
        {shown !== '' && pending === null && (
          <Button size="sm" variant="ghost" onClick={() => void apply('')}>
            Clear
          </Button>
        )}
      </div>
    </div>
  );
}
