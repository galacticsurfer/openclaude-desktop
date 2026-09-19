import { useEffect, useState } from 'react';
import { Download, HardDrive, Lock, Trash2 } from 'lucide-react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { useConversationStore } from '@/stores/useConversationStore';
import { Button } from '@/components/ui/Button';
import { Switch } from '@/components/ui/Switch';
import { Textarea } from '@/components/ui/Input';
import { Group, Row } from '../SettingsDialog';
import { formatBytes } from '@/lib/format';
import type { StorageStats } from '@/types';

export function PrivacyPanel() {
  const { settings, set } = useSettingsStore();
  const { toast, confirm } = useUIStore();
  const [stats, setStats] = useState<StorageStats | null>(null);
  const [pricingDraft, setPricingDraft] = useState<string | null>(null);

  useEffect(() => {
    void api.storageStats().then(setStats).catch(() => setStats(null));
  }, []);

  if (!settings) return null;

  async function exportEverything() {
    try {
      const json = await api.exportAllData();
      const { save } = await import('@tauri-apps/plugin-dialog');
      const path = await save({
        defaultPath: `openclaude-export-${new Date().toISOString().slice(0, 10)}.json`,
      });
      if (!path) return;
      await api.writeTextFile(path, json);
      toast('success', 'All data exported.');
    } catch (err) {
      toast('error', AppError.from(err).message);
    }
  }

  return (
    <>
      <section className="mb-7 rounded-lg border border-line bg-sunken/40 p-4">
        <div className="mb-2.5 flex items-center gap-2">
          <Lock size={15} className="text-success" aria-hidden />
          <h4 className="text-[13.5px] font-semibold text-ink">What leaves this computer</h4>
        </div>
        <dl className="space-y-2 text-[13px] leading-snug">
          <Fact term="Conversations" value="Stored only on this computer, in a local SQLite database." />
          <Fact
            term="API requests"
            value="Message text and attachments are sent to api.anthropic.com when you send a message, and nowhere else."
          />
          <Fact term="API key" value="Held in your system keyring. Never written to the database, a config file, or a log." />
          <Fact term="Telemetry" value="None. No analytics, no crash reporting, no update pings." />
          <Fact term="Logs" value="Diagnostics only — message content is never logged." />
        </dl>
      </section>

      <Group title="Cost estimates">
        <Switch
          checked={settings['privacy.showCost']}
          onChange={(v) => void set('privacy.showCost', v)}
          label="Show estimated cost"
          description="Off by default, because the prices below are a local copy that can go out of date. They are never fetched from anywhere — edit them to match your account."
        />
        {settings['privacy.showCost'] && (
          <div className="space-y-2 py-3">
            <p className="text-[12.5px] text-ink-faint">
              Prices per million tokens · as of {settings['privacy.pricing']?.asOf ?? 'unknown'}
            </p>
            <Textarea
              rows={10}
              value={pricingDraft ?? JSON.stringify(settings['privacy.pricing'], null, 2)}
              onChange={(e) => setPricingDraft(e.target.value)}
              spellCheck={false}
              aria-label="Pricing table"
              className="font-mono text-[12px]"
            />
            <div className="flex gap-2">
              <Button
                size="sm"
                variant="secondary"
                disabled={pricingDraft === null}
                onClick={() => {
                  try {
                    const parsed = JSON.parse(pricingDraft ?? '');
                    void set('privacy.pricing', parsed);
                    setPricingDraft(null);
                    toast('success', 'Pricing saved.');
                  } catch {
                    toast('error', 'That is not valid JSON.');
                  }
                }}
              >
                Save pricing
              </Button>
              {pricingDraft !== null && (
                <Button size="sm" variant="ghost" onClick={() => setPricingDraft(null)}>
                  Discard
                </Button>
              )}
            </div>
          </div>
        )}
      </Group>

      <Group title="Local data">
        {stats && (
          <div className="flex items-start gap-2.5 py-3 text-[13px]">
            <HardDrive size={15} className="mt-0.5 shrink-0 text-ink-faint" aria-hidden />
            <dl className="grid flex-1 grid-cols-2 gap-x-6 gap-y-1 text-ink-soft">
              <Stat label="Conversations" value={stats.conversationCount.toLocaleString()} />
              <Stat label="Messages" value={stats.messageCount.toLocaleString()} />
              <Stat label="Database" value={formatBytes(stats.databaseBytes)} />
              <Stat label="Attachments" value={`${stats.attachmentCount} · ${formatBytes(stats.attachmentBytes)}`} />
            </dl>
          </div>
        )}

        <Row
          label="Export everything"
          description="One JSON file with every conversation, message, project and setting. Never includes your API key."
          control={
            <Button variant="secondary" size="sm" onClick={() => void exportEverything()}>
              <Download size={13} /> Export data
            </Button>
          }
        />

        <Row
          label="Delete all conversations"
          description="Removes every conversation, message and attachment from this computer. Settings and your API key are kept."
          control={
            <Button
              variant="danger"
              size="sm"
              onClick={() =>
                confirm({
                  title: 'Delete all conversations?',
                  body: 'Every conversation, message and attachment will be permanently removed from this computer. This cannot be undone — export your data first if you may want it.',
                  confirmLabel: 'Delete everything',
                  destructive: true,
                  onConfirm: async () => {
                    await api.clearAllConversations();
                    await useConversationStore.getState().loadConversations();
                    useConversationStore.setState({ current: null, currentId: null, messages: [] });
                    toast('info', 'All conversations deleted.');
                    setStats(await api.storageStats());
                  },
                })
              }
            >
              <Trash2 size={13} /> Delete all
            </Button>
          }
        />
      </Group>
    </>
  );
}

function Fact({ term, value }: { term: string; value: string }) {
  return (
    <div className="flex gap-2">
      <dt className="w-28 shrink-0 font-medium text-ink">{term}</dt>
      <dd className="min-w-0 flex-1 text-ink-soft">{value}</dd>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-2">
      <dt>{label}</dt>
      <dd className="font-medium tabular-nums text-ink">{value}</dd>
    </div>
  );
}
