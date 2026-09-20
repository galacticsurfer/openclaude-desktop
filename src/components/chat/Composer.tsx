import { useCallback, useEffect, useRef, useState } from 'react';
import { ArrowUp, Paperclip, Square } from 'lucide-react';
import { useConversationStore } from '@/stores/useConversationStore';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { AttachmentChip } from './AttachmentChip';
import { SlashCommandMenu, matchCommands, slashQuery } from './SlashCommandMenu';
import { IconButton } from '@/components/ui/IconButton';
import { cn } from '@/lib/cn';

const MAX_ROWS_PX = 280;

export function Composer({ disabled }: { disabled?: boolean }) {
  const { pendingAttachments, send, stop, unstage, stagePaths, stageImage, isGenerating } =
    useConversationStore();
  const sendKey = useSettingsStore((s) => s.settings?.['general.sendKey'] ?? 'enter');
  const toast = useUIStore((s) => s.toast);

  const [value, setValue] = useState('');
  const slashCommands = useSettingsStore((s) => s.settings?.['claude.slashCommands'] ?? []);
  const [slashCursor, setSlashCursor] = useState(0);
  const slashOpen = slashQuery(value) !== null && matchCommands(slashCommands, slashQuery(value) ?? '').length > 0;

  const completeSlash = useCallback((command: string) => {
    // A trailing space both commits the choice and closes the menu.
    setValue(`/${command} `);
    textareaRef.current?.focus();
  }, []);
  const [busy, setBusy] = useState(false);
  // Dragging is reported by Tauri's native drag-drop event (see AppShell);
  // the DOM equivalent has no file paths, and handling both would attach a
  // dropped image twice.
  const dragging = useUIStore((s) => s.dragActive);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const generating = isGenerating();

  // Grow with content up to a cap, then scroll.
  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${Math.min(el.scrollHeight, MAX_ROWS_PX)}px`;
  }, [value]);

  // Focus the composer when the conversation changes.
  const currentId = useConversationStore((s) => s.currentId);
  useEffect(() => {
    if (!disabled) textareaRef.current?.focus();
  }, [currentId, disabled]);

  const submit = useCallback(async () => {
    const text = value.trim();
    if ((text === '' && pendingAttachments.length === 0) || generating || busy) return;

    setBusy(true);
    // Clear optimistically so typing the next message never waits on IPC,
    // but keep a copy to restore if the send is rejected.
    setValue('');
    try {
      await send(text);
    } catch {
      setValue(text);
    } finally {
      setBusy(false);
      textareaRef.current?.focus();
    }
  }, [value, pendingAttachments.length, generating, busy, send]);

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    // The slash menu owns navigation keys while it is open.
    if (slashOpen) {
      const matches = matchCommands(slashCommands, slashQuery(value) ?? '');
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSlashCursor((c) => Math.min(c + 1, matches.length - 1));
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSlashCursor((c) => Math.max(c - 1, 0));
        return;
      }
      if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey)) {
        const picked = matches[slashCursor];
        if (picked) {
          e.preventDefault();
          completeSlash(picked);
          return;
        }
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        // Close the menu without discarding what was typed.
        setValue((v) => `${v} `);
        return;
      }
    }

    if (e.key === 'Escape' && generating) {
      e.preventDefault();
      void stop();
      return;
    }
    if (e.key !== 'Enter' || e.nativeEvent.isComposing) return;

    const wantsSend =
      sendKey === 'enter' ? !e.shiftKey && !e.ctrlKey && !e.metaKey : e.ctrlKey || e.metaKey;

    if (wantsSend) {
      e.preventDefault();
      void submit();
    }
  }

  async function pickFiles() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const picked = await open({ multiple: true, directory: false });
      if (!picked) return;
      await stagePaths(Array.isArray(picked) ? picked : [picked]);
    } catch {
      toast('error', 'Could not open the file picker.');
    }
  }

  // Paste an image straight from the clipboard.
  async function onPaste(e: React.ClipboardEvent) {
    const items = [...e.clipboardData.items].filter((i) => i.type.startsWith('image/'));
    if (items.length === 0) return;
    e.preventDefault();

    for (const item of items) {
      const file = item.getAsFile();
      if (!file) continue;
      const buf = await file.arrayBuffer();
      const base64 = bytesToBase64(new Uint8Array(buf));
      const ext = file.type.split('/')[1] ?? 'png';
      await stageImage(file.name || `pasted-image.${ext}`, file.type, base64);
    }
  }

  const canSend = (value.trim() !== '' || pendingAttachments.length > 0) && !generating && !busy;

  return (
    <div className="border-t border-line bg-canvas px-4 pb-4 pt-3">
      <div className="mx-auto w-full max-w-3xl">
        {pendingAttachments.length > 0 && (
          <div className="mb-2 flex flex-wrap gap-1.5">
            {pendingAttachments.map((a) => (
              <AttachmentChip key={a.id} attachment={a} onRemove={() => void unstage(a.id)} />
            ))}
          </div>
        )}

        <div
          className={cn(
            'relative flex items-end gap-1.5 rounded-xl border bg-surface p-1.5 shadow-subtle transition-colors',
            dragging ? 'border-accent ring-2 ring-accent/20' : 'border-line-strong',
            'focus-within:border-accent/60',
          )}
        >
          <IconButton
            label="Attach files"
            onClick={pickFiles}
            disabled={disabled}
            className="mb-0.5"
          >
            <Paperclip size={16} />
          </IconButton>

          <textarea
            ref={textareaRef}
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={onKeyDown}
            onPaste={onPaste}
            rows={1}
            disabled={disabled}
            aria-label="Message Claude"
            // Kept short: a longer hint clips in a narrow window, and the
            // send key is discoverable via Ctrl+/ and Settings → General.
            placeholder={
              disabled
                ? 'Claude Code is not installed — see Settings'
                : slashCommands.length > 0
                  ? 'Message Claude…  (/ for commands)'
                  : 'Message Claude…'
            }
            title={sendKey === 'enter' ? 'Enter to send · Shift+Enter for a new line' : 'Ctrl+Enter to send'}
            className="min-h-[36px] flex-1 resize-none bg-transparent py-2 text-[14.5px] leading-relaxed text-ink outline-none placeholder:text-ink-faint disabled:cursor-not-allowed scroll-thin"
            style={{ maxHeight: MAX_ROWS_PX }}
          />

          {generating ? (
            <button
              type="button"
              onClick={() => void stop()}
              aria-label="Stop generating (Esc)"
              title="Stop generating (Esc)"
              className="mb-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border border-line-strong bg-surface text-ink-soft transition-colors hover:border-danger/50 hover:text-danger"
            >
              <Square size={13} fill="currentColor" />
            </button>
          ) : (
            <button
              type="button"
              onClick={() => void submit()}
              disabled={!canSend}
              aria-label="Send message"
              title="Send message"
              className={cn(
                'mb-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg transition-colors',
                canSend
                  ? 'bg-accent text-on-accent hover:bg-accent-hover'
                  : 'bg-sunken text-ink-faint',
              )}
            >
              <ArrowUp size={16} strokeWidth={2.5} />
            </button>
          )}

          <SlashCommandMenu
            value={value}
            commands={slashCommands}
            cursor={slashCursor}
            onCursorChange={setSlashCursor}
            onPick={completeSlash}
          />

          {dragging && (
            <div className="pointer-events-none absolute inset-0 flex items-center justify-center rounded-xl bg-accent-soft/90 text-[13px] font-medium text-accent">
              Drop files to attach
            </div>
          )}
        </div>

        <p className="mt-1.5 px-1 text-center text-[11px] text-ink-faint">
          Claude can make mistakes. Verify important information.
        </p>
      </div>
    </div>
  );
}

/** Chunked so a multi-megabyte image does not blow the argument limit. */
function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}
