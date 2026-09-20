import { useCallback, useEffect, useState } from 'react';
import { ArrowDown, Info, MessageSquare, PanelLeft, Sparkles } from 'lucide-react';
import { useConversationStore } from '@/stores/useConversationStore';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { useAutoScroll } from '@/hooks/useAutoScroll';
import { MessageBubble } from './MessageBubble';
import { Composer } from './Composer';
import { ModelSelector } from './ModelSelector';
import { Empty } from '@/components/ui/Empty';
import appIcon from '@/assets/openclaude.png';
import { IconButton } from '@/components/ui/IconButton';
import { Spinner } from '@/components/ui/Spinner';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';

export function ChatView() {
  const {
    current, messages, streams, thinkingState, loadingMessages, currentId,
    retry, continueReply, newConversation, open, loadConversations, projects,
  } = useConversationStore();
  const { toggleSidebar, sidebarCollapsed, openOverlay, toast } = useUIStore();
  const claudeCode = useSettingsStore((s) => s.claudeCode);
  const [renaming, setRenaming] = useState(false);
  const [draftTitle, setDraftTitle] = useState('');

  const streamKey = Object.values(streams)
    .map((s) => s.text.length + s.thinking.length)
    .join(',');
  const { ref: scrollRef, atBottom, scrollToBottom } = useAutoScroll<HTMLDivElement>([
    messages.length,
    streamKey,
    currentId,
  ]);

  const copyText = useCallback(
    async (text: string) => {
      try {
        const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
        await writeText(text);
      } catch {
        toast('error', 'Could not copy to the clipboard.');
      }
    },
    [toast],
  );

  const branch = useCallback(
    async (messageId: string) => {
      try {
        const created = await api.branchConversation(messageId);
        await loadConversations();
        await open(created.id);
        toast('success', 'Branched into a new conversation.');
      } catch (err) {
        toast('error', AppError.from(err).message);
      }
    },
    [loadConversations, open, toast],
  );

  useEffect(() => {
    setRenaming(false);
  }, [currentId]);

  if (!current) {
    return (
      <div className="flex h-full flex-col">
        <Header onToggleSidebar={toggleSidebar} collapsed={sidebarCollapsed} />
        <div className="flex flex-1 items-center justify-center">
          <Empty
            image={appIcon}
            icon={MessageSquare}
            title="No conversation open"
            body="Pick one from the sidebar, or start a new chat."
            action={
              <button
                type="button"
                onClick={() => void newConversation()}
                className="rounded-md bg-accent px-3.5 py-2 text-[13.5px] font-medium text-on-accent hover:bg-accent-hover"
              >
                New chat
              </button>
            }
          />
        </div>
      </div>
    );
  }

  const project = projects.find((p) => p.id === current.projectId);
  const lastIndex = messages.length - 1;

  return (
    <div className="flex h-full flex-col">
      <Header onToggleSidebar={toggleSidebar} collapsed={sidebarCollapsed}>
        <div className="flex min-w-0 flex-1 items-center gap-2">
          {renaming ? (
            <input
              autoFocus
              value={draftTitle}
              onChange={(e) => setDraftTitle(e.target.value)}
              onBlur={() => setRenaming(false)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  const t = draftTitle.trim();
                  if (t) {
                    void api.renameConversation(current.id, t).then(() => {
                      void loadConversations();
                      void open(current.id);
                    });
                  }
                  setRenaming(false);
                } else if (e.key === 'Escape') {
                  e.stopPropagation();
                  setRenaming(false);
                }
              }}
              aria-label="Conversation title"
              className="min-w-0 flex-1 rounded border border-line-strong bg-surface px-2 py-0.5 text-[14px] font-medium text-ink"
            />
          ) : (
            <button
              type="button"
              onDoubleClick={() => {
                setDraftTitle(current.title);
                setRenaming(true);
              }}
              title="Double-click to rename"
              className="min-w-0 truncate text-left text-[14px] font-medium tracking-[-0.01em] text-ink"
            >
              {current.title}
            </button>
          )}

          {project && (
            <span
              className="shrink-0 rounded border border-line bg-sunken px-1.5 py-0.5 text-[11px] text-ink-soft"
              title={
                project.instructions
                  ? `Project instructions are active:\n\n${project.instructions.slice(0, 400)}`
                  : 'Project'
              }
            >
              {project.name}
              {project.instructions.trim() !== '' && (
                <Sparkles size={9} className="ml-1 inline text-accent" aria-label="instructions active" />
              )}
            </span>
          )}
        </div>

        <ModelSelector
          value={current.model}
          onChange={(model) => {
            void api.setConversationModel(current.id, model).then(() => open(current.id));
          }}
        />
        <IconButton
          label="Conversation details"
          onClick={() => openOverlay({ kind: 'conversationInfo', conversationId: current.id })}
        >
          <Info size={15} />
        </IconButton>
      </Header>

      {project?.instructions.trim() && (
        <div className="border-b border-line bg-accent-soft/40 px-4 py-1.5">
          <p className="mx-auto max-w-3xl text-[12px] text-ink-soft">
            <Sparkles size={11} className="mr-1.5 inline text-accent" aria-hidden />
            Project instructions from <strong className="font-medium">{project.name}</strong> are
            included in every message in this conversation.
          </p>
        </div>
      )}

      <div ref={scrollRef} className="relative min-h-0 flex-1 overflow-y-auto scroll-thin">
        <div className="mx-auto w-full max-w-3xl px-4 py-6">
          {loadingMessages && messages.length === 0 ? (
            <div className="flex justify-center py-12">
              <Spinner label="Loading messages" />
            </div>
          ) : messages.length === 0 ? (
            <Empty
              icon={MessageSquare}
              title="Start the conversation"
              body={
                claudeCode?.installed
                  ? 'Type below, or drop in a file to discuss.'
                  : 'Claude Code is not installed — see Settings → Claude.'
              }
            />
          ) : (
            <div className="space-y-6">
              {messages.map((m, i) => (
                <MessageBubble
                  key={m.id}
                  message={m}
                  stream={streams[m.id] ?? null}
                  thinkingState={thinkingState[m.id] ?? null}
                  isLast={i === lastIndex}
                  onRetry={() => void retry()}
                  onContinue={() => void continueReply()}
                  onBranch={() => void branch(m.id)}
                  onCopy={(t) => void copyText(t)}
                />
              ))}
            </div>
          )}
        </div>

        {!atBottom && messages.length > 0 && (
          <button
            type="button"
            onClick={() => scrollToBottom()}
            aria-label="Scroll to the latest message"
            className="sticky bottom-4 left-1/2 flex size-8 -translate-x-1/2 items-center justify-center rounded-full border border-line bg-raised text-ink-soft shadow-raised hover:text-ink"
          >
            <ArrowDown size={15} />
          </button>
        )}
      </div>

      <Composer disabled={!claudeCode?.installed} />
    </div>
  );
}

function Header({
  children, onToggleSidebar, collapsed,
}: {
  children?: React.ReactNode;
  onToggleSidebar: () => void;
  collapsed: boolean;
}) {
  return (
    <header
      // Opaque on purpose: a translucent blurred header has to be re-blurred
      // every frame while the window resizes, and over a solid canvas it
      // looks identical to this.
      className="flex h-12 shrink-0 items-center gap-2 border-b border-line bg-canvas px-3"
    >
      <IconButton
        label={collapsed ? 'Show sidebar (Ctrl B)' : 'Hide sidebar (Ctrl B)'}
        onClick={onToggleSidebar}
      >
        <PanelLeft size={16} />
      </IconButton>
      {children}
    </header>
  );
}
