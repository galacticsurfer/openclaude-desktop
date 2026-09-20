import { useEffect, useState } from 'react';
import { Download } from 'lucide-react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useConversationStore } from '@/stores/useConversationStore';
import { useUIStore } from '@/stores/useUIStore';
import { exportConversation } from '@/hooks/useConversationActions';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import { Field, Textarea } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { formatFull, formatTokens } from '@/lib/format';
import type { Conversation, UsageTotals } from '@/types';

export function ConversationInfoDialog({ conversationId }: { conversationId: string }) {
  const { closeOverlay, toast } = useUIStore();
  const { projects, loadConversations, open, currentId } = useConversationStore();

  const [conversation, setConversation] = useState<Conversation | null>(null);
  const [usage, setUsage] = useState<UsageTotals | null>(null);
  const [systemPrompt, setSystemPrompt] = useState('');
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    void Promise.all([
      api.getConversation(conversationId),
      api.conversationUsage(conversationId),
    ])
      .then(([c, u]) => {
        setConversation(c);
        setUsage(u);
        setSystemPrompt(c.systemPrompt ?? '');
      })
      .catch((e) => toast('error', AppError.from(e).message));
  }, [conversationId, toast]);

  if (!conversation) return null;

  async function refresh() {
    await loadConversations();
    if (currentId === conversationId) await open(conversationId);
  }

  return (
    <Dialog
      open
      onClose={closeOverlay}
      title={conversation.title}
      description={`Created ${formatFull(conversation.createdAt)}`}
      size="lg"
      footer={
        <>
          <Button
            variant="ghost"
            className="mr-auto"
            onClick={() => void exportConversation(conversationId, 'markdown', toast)}
          >
            <Download size={14} /> Export Markdown
          </Button>
          <Button variant="ghost" onClick={closeOverlay}>
            Close
          </Button>
          {dirty && (
            <Button
              variant="primary"
              onClick={() => {
                void api
                  .setConversationSystemPrompt(conversationId, systemPrompt.trim() || null)
                  .then(refresh)
                  .then(() => {
                    setDirty(false);
                    toast('success', 'Saved.');
                  })
                  .catch((e) => toast('error', AppError.from(e).message));
              }}
            >
              Save
            </Button>
          )}
        </>
      }
    >
      <div className="space-y-5 p-5">
        <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-[13px]">
          <Stat label="Model" value={conversation.model} />
          <Stat label="Messages" value={String(usage?.messageCount ?? 0)} />
          <Stat label="Input tokens" value={formatTokens(usage?.inputTokens ?? 0)} />
          <Stat label="Output tokens" value={formatTokens(usage?.outputTokens ?? 0)} />
          {(usage?.cacheReadTokens ?? 0) > 0 && (
            <Stat label="Cache reads" value={formatTokens(usage?.cacheReadTokens ?? 0)} />
          )}
          <Stat label="Last activity" value={formatFull(conversation.lastMessageAt ?? conversation.updatedAt)} />
        </dl>

        {conversation.branchedFromMessageId && (
          <p className="rounded-md border border-line bg-sunken/60 px-3 py-2 text-[12.5px] text-ink-soft">
            This conversation was branched from another one.
          </p>
        )}

        <Field label="Project" hint="Moving a conversation applies that project's instructions to it.">
          <Select
            value={conversation.projectId ?? ''}
            onChange={(e) => {
              void api
                .setConversationProject(conversationId, e.target.value || null)
                .then(refresh)
                .then(() => api.getConversation(conversationId))
                .then(setConversation)
                .catch((err) => toast('error', AppError.from(err).message));
            }}
          >
            <option value="">No project</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </Select>
        </Field>

        <Field
          label="Conversation instructions"
          hint="Added to the system prompt for this conversation only, after any project instructions."
        >
          <Textarea
            rows={4}
            value={systemPrompt}
            onChange={(e) => {
              setSystemPrompt(e.target.value);
              setDirty(true);
            }}
            placeholder="Answer concisely and show your reasoning for any trade-off."
          />
        </Field>
      </div>
    </Dialog>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-3 border-b border-line pb-1.5">
      <dt className="text-ink-faint">{label}</dt>
      <dd className="truncate font-medium text-ink" title={value}>{value}</dd>
    </div>
  );
}
