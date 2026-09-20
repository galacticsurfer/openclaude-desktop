import { useEffect } from 'react';

export interface Hotkey {
  /** Lowercase `event.key`, or a `Digit`/`Key` code for layout independence. */
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  run: (e: KeyboardEvent) => void;
  /** Fire even while a text field has focus (Escape, Ctrl+K…). */
  allowInInput?: boolean;
}

function isTextEntry(el: EventTarget | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  return (
    el.tagName === 'INPUT' ||
    el.tagName === 'TEXTAREA' ||
    el.tagName === 'SELECT' ||
    el.isContentEditable
  );
}

/**
 * Global keyboard shortcuts.
 *
 * Registered on `document` in the capture phase so a shortcut still works
 * when focus is inside the composer, but individual bindings opt in to that
 * via `allowInInput` — plain letter keys must never steal typed characters.
 */
export function useHotkeys(hotkeys: Hotkey[], enabled = true): void {
  useEffect(() => {
    if (!enabled) return;

    function onKeyDown(e: KeyboardEvent) {
      // Ignore the synthetic keydown IMEs emit while composing.
      if (e.isComposing) return;

      const inInput = isTextEntry(e.target);
      for (const hk of hotkeys) {
        if (e.key.toLowerCase() !== hk.key.toLowerCase()) continue;
        if (Boolean(hk.ctrl) !== (e.ctrlKey || e.metaKey)) continue;
        if (Boolean(hk.shift) !== e.shiftKey) continue;
        if (Boolean(hk.alt) !== e.altKey) continue;
        if (inInput && !hk.allowInInput) continue;

        e.preventDefault();
        e.stopPropagation();
        hk.run(e);
        return;
      }
    }

    document.addEventListener('keydown', onKeyDown, true);
    return () => document.removeEventListener('keydown', onKeyDown, true);
  }, [hotkeys, enabled]);
}

/** Canonical list, shown in the shortcuts dialog and the command palette. */
export const SHORTCUTS: Array<{ keys: string; label: string; group: string }> = [
  { keys: 'Ctrl+N', label: 'New conversation', group: 'General' },
  { keys: 'Ctrl+K', label: 'Search conversations', group: 'General' },
  { keys: 'Ctrl+Shift+P', label: 'Command palette', group: 'General' },
  { keys: 'Ctrl+,', label: 'Settings', group: 'General' },
  { keys: 'Ctrl+/', label: 'Keyboard shortcuts', group: 'General' },
  { keys: 'Ctrl+B', label: 'Toggle sidebar', group: 'View' },
  { keys: 'Ctrl+W', label: 'Close tab', group: 'View' },
  { keys: 'Ctrl+`', label: 'Toggle terminal', group: 'View' },
  { keys: 'Ctrl+Shift+N', label: 'New project', group: 'General' },
  { keys: 'Ctrl+L', label: 'Focus conversation list', group: 'View' },
  { keys: 'Enter', label: 'Send message', group: 'Composer' },
  { keys: 'Shift+Enter', label: 'New line', group: 'Composer' },
  { keys: 'Esc', label: 'Stop generating / close overlay', group: 'Composer' },
  { keys: 'Ctrl+Shift+C', label: 'Copy last reply', group: 'Conversation' },
  { keys: 'Ctrl+F', label: 'Find in conversation', group: 'Conversation' },
  { keys: 'Ctrl+Shift+L', label: 'Prompt library', group: 'General' },
  { keys: 'Enter', label: 'Next match, while finding', group: 'Conversation' },
  { keys: 'Shift+Enter', label: 'Previous match, while finding', group: 'Conversation' },
];
