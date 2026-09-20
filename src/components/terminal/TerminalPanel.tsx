import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { X } from 'lucide-react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import * as api from '@/services/api';
import { useUIStore } from '@/stores/useUIStore';
import { useConversationStore } from '@/stores/useConversationStore';
import { IconButton } from '@/components/ui/IconButton';

/**
 * A real shell, driven by you.
 *
 * Claude cannot see this, type into it, or know it exists. That is
 * structural rather than a promise: no IPC command carries terminal output
 * into a prompt, and nothing in the chat path can reach a shell session.
 * Moving something from here into a conversation means selecting and
 * copying it, deliberately.
 *
 * It opens in the project's working folder when the conversation has one,
 * and otherwise in the same empty scratch directory conversations use —
 * never wherever the app happened to be launched from.
 */
export function TerminalPanel() {
  const open = useUIStore((s) => s.terminalOpen);
  const close = useUIStore((s) => s.closeTerminal);
  const projectId = useConversationStore((s) => s.current?.projectId ?? null);
  const hostRef = useRef<HTMLDivElement>(null);
  const idRef = useRef(`term-${Math.random().toString(36).slice(2)}`);

  useEffect(() => {
    if (!open || !hostRef.current) return;
    const id = idRef.current;
    let disposed = false;

    const term = new Terminal({
      fontSize: 13,
      fontFamily: 'var(--font-mono), monospace',
      cursorBlink: true,
      // The app owns the surrounding chrome; let the terminal inherit it.
      theme: { background: 'rgba(0,0,0,0)' },
      allowTransparency: true,
      scrollback: 5000,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(hostRef.current);
    fit.fit();

    term.onData((data) => void api.shellWrite(id, data));

    const resize = () => {
      fit.fit();
      void api.shellResize(id, term.rows, term.cols);
    };
    const observer = new ResizeObserver(resize);
    observer.observe(hostRef.current);

    const unlisten = listen<{ id: string; data: string }>('shell:data', (e) => {
      if (e.payload.id === id) term.write(e.payload.data);
    });

    void api.shellOpen(id, projectId).then(() => {
      if (!disposed) resize();
    });
    term.focus();

    return () => {
      disposed = true;
      observer.disconnect();
      void unlisten.then((un) => un());
      // Kills the child process; a hidden panel must not leave a shell
      // running in the background.
      void api.shellClose(id);
      term.dispose();
    };
  }, [open, projectId]);

  if (!open) return null;

  return (
    <section
      aria-label="Terminal"
      className="flex h-64 shrink-0 flex-col border-t border-line bg-sunken"
    >
      <header className="flex items-center gap-2 border-b border-line px-3 py-1">
        <span className="font-mono text-[11px] uppercase tracking-wide text-ink-faint">
          Terminal
        </span>
        <span className="text-[11.5px] text-ink-faint">
          Yours, not Claude's — nothing here is visible to the conversation.
        </span>
        <IconButton label="Close terminal (Ctrl `)" size="sm" className="ml-auto" onClick={close}>
          <X size={14} />
        </IconButton>
      </header>
      <div ref={hostRef} className="min-h-0 flex-1 px-2 py-1" />
    </section>
  );
}
