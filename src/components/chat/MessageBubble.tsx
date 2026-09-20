import { memo, useState } from 'react';
import {
  AlertTriangle, Brain, Check, ChevronDown, ChevronRight, Copy, GitBranch,
  PauseCircle, Pencil, RefreshCw, Play, Wrench,
} from 'lucide-react';
import type { Message, ToolCallRecord } from '@/types';
import { Markdown } from '@/components/markdown/Markdown';
import { useSmoothText } from '@/hooks/useSmoothText';
import { StreamingMarkdown } from '@/components/markdown/StreamingMarkdown';
import { IconButton } from '@/components/ui/IconButton';
import { Button } from '@/components/ui/Button';
import { formatTime, formatTokens } from '@/lib/format';
import { cn } from '@/lib/cn';
import { AttachmentChip } from './AttachmentChip';

interface Props {
  message: Message;
  /** Live text when this message is still streaming, else null. */
  stream: { text: string; thinking: string } | null;
  /** Live reasoning state, present only while Claude is thinking. */
  thinkingState: { tokens: number | null } | null;
  /** True when this is the match the find bar is currently sitting on. */
  matched?: boolean;
  /** Tool calls in flight, while this reply is live. */
  toolState?: ToolCallRecord[];
  isLast: boolean;
  onRetry: () => void;
  onContinue: () => void;
  onBranch: () => void;
  onEdit: (text: string) => void;
  onCopy: (text: string) => void;
}

/**
 * One turn in the conversation.
 *
 * User turns get a tinted card; Claude's are set flush on the canvas so long
 * answers read like a document rather than a chat bubble.
 */
export const MessageBubble = memo(function MessageBubble({
  message: m,
  stream,
  thinkingState,
  matched = false,
  toolState,
  isLast,
  onRetry,
  onContinue,
  onBranch,
  onEdit,
  onCopy,
}: Props) {
  const [copied, setCopied] = useState(false);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState('');
  const [showThinking, setShowThinking] = useState(false);
  const [showDetails, setShowDetails] = useState(false);

  const streaming = stream !== null;
  // Deltas arrive in ~25-character lumps; reveal them at frame rate so the
  // reply flows instead of jumping. See useSmoothText.
  const text = useSmoothText(streaming ? stream.text : m.content, streaming);
  const thinking = streaming ? stream.thinking : (m.thinking ?? '');
  const isUser = m.role === 'user';
  // Claude Code reports reasoning as a token estimate and withholds the text,
  // so the count is usually all there is to show. Live state first, then the
  // number persisted on the message once the reply is finished.
  const thinkingTokens = thinkingState?.tokens ?? m.thinkingTokens;
  const reasoning = thinkingState !== null || (m.thinkingTokens ?? 0) > 0;
  // Live while streaming, then from what was stored with the message.
  const toolCalls: ToolCallRecord[] =
    toolState ??
    (Array.isArray((m.metadata as { toolCalls?: unknown })?.toolCalls)
      ? ((m.metadata as { toolCalls: ToolCallRecord[] }).toolCalls)
      : []);

  function copy() {
    onCopy(text);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  }

  return (
    <article
      data-message-id={m.id}
      className={cn(
        'group/msg message-block relative',
        isUser ? 'pl-8' : '',
        // Offset the ring so it frames the message instead of touching it.
        matched && 'rounded-lg outline outline-2 outline-offset-4 outline-accent',
      )}
      aria-label={`${isUser ? 'You' : 'Claude'} at ${formatTime(m.createdAt)}`}
    >
      <header className="mb-1.5 flex items-center gap-2">
        <span
          className={cn(
            'text-[12px] font-semibold tracking-wide',
            isUser ? 'text-ink-soft' : 'text-accent',
          )}
        >
          {isUser ? 'You' : 'Claude'}
        </span>
        <span className="text-[11px] tabular-nums text-ink-faint opacity-0 transition-opacity group-hover/msg:opacity-100">
          {formatTime(m.createdAt)}
        </span>

        <div className="ml-auto flex items-center gap-0.5 opacity-0 transition-opacity focus-within:opacity-100 group-hover/msg:opacity-100">
          {text.length > 0 && (
            <IconButton label={copied ? 'Copied' : 'Copy message'} size="sm" onClick={copy}>
              {copied ? <Check size={13} className="text-success" /> : <Copy size={13} />}
            </IconButton>
          )}
          {isUser && !streaming && !editing && (
            <IconButton
              label="Edit and send again"
              size="sm"
              onClick={() => {
                setDraft(m.content);
                setEditing(true);
              }}
            >
              <Pencil size={13} />
            </IconButton>
          )}
          {!streaming && (
            <IconButton label="Branch a new conversation from here" size="sm" onClick={onBranch}>
              <GitBranch size={13} />
            </IconButton>
          )}
        </div>
      </header>

      {m.attachments.length > 0 && (
        <div className="mb-2 flex flex-wrap gap-1.5">
          {m.attachments.map((a) => (
            <AttachmentChip key={a.id} attachment={a} readOnly />
          ))}
        </div>
      )}

      {reasoning && thinking.length === 0 && (
        <div className="mb-2 flex items-center gap-1.5 text-[12px] text-ink-faint">
          {thinkingState !== null ? (
            <>
              <span
                className="size-3 animate-spin rounded-full border-[1.5px] border-current border-r-transparent"
                aria-hidden
              />
              <span>
                Thinking{thinkingTokens ? ` · ~${formatTokens(thinkingTokens)} tokens` : '…'}
              </span>
            </>
          ) : (
            <>
              <Brain size={12} aria-hidden />
              <span>Thought for ~{formatTokens(thinkingTokens ?? 0)} tokens</span>
            </>
          )}
        </div>
      )}

      {toolCalls.length > 0 && (
        <ul className="mb-2 space-y-1">
          {toolCalls.map((t) => (
            <li
              key={t.id}
              className="flex items-center gap-1.5 rounded-md border border-line bg-sunken/60 px-2.5 py-1 text-[12px]"
            >
              <Wrench size={12} className="shrink-0 text-ink-faint" aria-hidden />
              <code className="min-w-0 flex-1 truncate font-mono text-ink-soft">{t.name}</code>
              {t.ok === undefined ? (
                <span className="text-ink-faint">running…</span>
              ) : t.ok ? (
                <Check size={12} className="text-success" aria-label="succeeded" />
              ) : (
                <AlertTriangle size={12} className="text-danger" aria-label="failed" />
              )}
            </li>
          ))}
        </ul>
      )}

      {thinking.length > 0 && (
        <div className="mb-2 overflow-hidden rounded-md border border-line bg-sunken/60">
          <button
            type="button"
            onClick={() => setShowThinking((v) => !v)}
            aria-expanded={showThinking}
            className="flex w-full items-center gap-1.5 px-2.5 py-1.5 text-left text-[12px] text-ink-faint hover:text-ink-soft"
          >
            {showThinking ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
            <Brain size={12} aria-hidden />
            Thinking
            {streaming && !showThinking && <span className="ml-1 animate-pulse">…</span>}
          </button>
          {showThinking && (
            <pre className="max-h-64 overflow-auto scroll-thin whitespace-pre-wrap border-t border-line px-3 py-2 font-mono text-[12px] leading-relaxed text-ink-soft">
              {thinking}
            </pre>
          )}
        </div>
      )}

      {editing ? (
        <div className="rounded-lg border border-accent bg-surface p-2 shadow-subtle">
          <textarea
            value={draft}
            autoFocus
            rows={Math.min(12, draft.split('\n').length + 1)}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                e.preventDefault();
                setEditing(false);
              } else if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                if (draft.trim() !== '') {
                  setEditing(false);
                  onEdit(draft);
                }
              }
            }}
            aria-label="Edit your message"
            className="w-full resize-none bg-transparent px-1.5 py-1 text-[14.5px] leading-relaxed text-ink outline-none scroll-thin"
          />
          <div className="mt-1 flex items-center justify-end gap-2 px-1.5">
            <span className="mr-auto text-[11.5px] text-ink-faint">
              Replies after this point will be discarded.
            </span>
            <Button size="sm" variant="ghost" onClick={() => setEditing(false)}>
              Cancel
            </Button>
            <Button
              size="sm"
              disabled={draft.trim() === ''}
              onClick={() => {
                setEditing(false);
                onEdit(draft);
              }}
            >
              Send
            </Button>
          </div>
        </div>
      ) : (
      <div
        className={cn(
          isUser &&
            'rounded-lg border border-line bg-surface px-3.5 py-2.5 shadow-subtle',
        )}
      >
        {text.length > 0 ? (
          // While streaming, parse only completed blocks; see
          // StreamingMarkdown for why the whole-string path is O(n²).
          streaming ? (
            <StreamingMarkdown caret>{text}</StreamingMarkdown>
          ) : (
            <Markdown>{text}</Markdown>
          )
        ) : streaming ? (
          <div className="flex items-center gap-2 py-1 text-[13px] text-ink-faint">
            <span className="flex gap-1" aria-hidden>
              <Dot delay="0ms" />
              <Dot delay="160ms" />
              <Dot delay="320ms" />
            </span>
            <span className="sr-only">Claude is responding</span>
          </div>
        ) : null}

      </div>

      )}

      {m.status === 'interrupted' && !streaming && (
        <Notice
          tone="warn"
          icon={<PauseCircle size={14} />}
          title="This reply was interrupted."
          body="It stopped before Claude finished — either you pressed Stop, or the app closed."
          actions={
            <>
              {isLast && (
                <Button size="sm" variant="secondary" onClick={onContinue}>
                  <Play size={13} /> Continue
                </Button>
              )}
              {isLast && (
                <Button size="sm" variant="ghost" onClick={onRetry}>
                  <RefreshCw size={13} /> Retry
                </Button>
              )}
            </>
          }
        />
      )}

      {m.status === 'error' && !streaming && (
        <Notice
          tone="danger"
          icon={<AlertTriangle size={14} />}
          title={m.errorMessage ?? 'Claude could not complete this response.'}
          actions={
            <>
              {isLast && (
                <Button size="sm" variant="secondary" onClick={onRetry}>
                  <RefreshCw size={13} /> Retry
                </Button>
              )}
              <Button size="sm" variant="ghost" onClick={() => setShowDetails((v) => !v)}>
                Details
              </Button>
            </>
          }
        >
          {showDetails && (
            <dl className="mt-2 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 font-mono text-[11.5px] text-ink-faint">
              <dt>kind</dt>
              <dd>{m.errorKind ?? '—'}</dd>
              <dt>model</dt>
              <dd>{m.model ?? '—'}</dd>
              <dt>time</dt>
              <dd>{new Date(m.updatedAt).toISOString()}</dd>
            </dl>
          )}
        </Notice>
      )}

      {!streaming && m.role === 'assistant' && (m.outputTokens ?? 0) > 0 && (
        <p className="mt-1.5 text-[11px] tabular-nums text-ink-faint opacity-0 transition-opacity group-hover/msg:opacity-100">
          {formatTokens(m.inputTokens ?? 0)} in · {formatTokens(m.outputTokens ?? 0)} out
          {m.model && ` · ${m.model}`}
          {m.stopReason === 'max_tokens' && (
            <span className="ml-2 text-warn">hit the response length limit</span>
          )}
        </p>
      )}
    </article>
  );
});

function Dot({ delay }: { delay: string }) {
  return (
    <span
      className="size-1.5 animate-pulse rounded-full bg-ink-faint"
      style={{ animationDelay: delay }}
    />
  );
}

function Notice({
  tone, icon, title, body, actions, children,
}: {
  tone: 'warn' | 'danger';
  icon: React.ReactNode;
  title: string;
  body?: string;
  actions?: React.ReactNode;
  children?: React.ReactNode;
}) {
  return (
    <div
      role="status"
      className={cn(
        'mt-2 rounded-md border px-3 py-2',
        tone === 'danger' ? 'border-danger/30 bg-danger-soft' : 'border-warn/30 bg-warn/5',
      )}
    >
      <div className="flex items-start gap-2">
        <span className={cn('mt-0.5 shrink-0', tone === 'danger' ? 'text-danger' : 'text-warn')} aria-hidden>
          {icon}
        </span>
        <div className="min-w-0 flex-1">
          <p className="text-[13px] font-medium text-ink">{title}</p>
          {body && <p className="mt-0.5 text-[12.5px] leading-snug text-ink-soft">{body}</p>}
          {children}
        </div>
      </div>
      {actions && <div className="mt-2 flex items-center gap-1.5 pl-6">{actions}</div>}
    </div>
  );
}
