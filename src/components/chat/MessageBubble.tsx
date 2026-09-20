import { memo, useState } from 'react';
import {
  AlertTriangle, Brain, Check, ChevronDown, ChevronRight, Copy, GitBranch,
  PauseCircle, RefreshCw, Play,
} from 'lucide-react';
import type { Message } from '@/types';
import { Markdown } from '@/components/markdown/Markdown';
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
  isLast: boolean;
  onRetry: () => void;
  onContinue: () => void;
  onBranch: () => void;
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
  isLast,
  onRetry,
  onContinue,
  onBranch,
  onCopy,
}: Props) {
  const [copied, setCopied] = useState(false);
  const [showThinking, setShowThinking] = useState(false);
  const [showDetails, setShowDetails] = useState(false);

  const streaming = stream !== null;
  const text = streaming ? stream.text : m.content;
  const thinking = streaming ? stream.thinking : (m.thinking ?? '');
  const isUser = m.role === 'user';

  function copy() {
    onCopy(text);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  }

  return (
    <article
      className={cn('group/msg message-block relative', isUser ? 'pl-8' : '')}
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
            <StreamingMarkdown>{text}</StreamingMarkdown>
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

        {streaming && text.length > 0 && <span className="stream-caret animate-caret" aria-hidden />}
      </div>

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
