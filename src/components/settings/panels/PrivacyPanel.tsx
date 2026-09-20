import { useEffect, useState } from 'react';
import { Download, HardDrive, Lock, Trash2 } from 'lucide-react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { useConversationStore } from '@/stores/useConversationStore';
import { Button } from '@/components/ui/Button';
import { Group, Row } from '../SettingsDialog';
import { formatBytes } from '@/lib/format';
import type { StorageStats } from '@/types';

export function PrivacyPanel() {
  const { settings } = useSettingsStore();
  const { toast, confirm } = useUIStore();
  const [stats, setStats] = useState<StorageStats | null>(null);

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
            term="Requests"
            value="Sent through the Claude Code CLI running on this machine, under your own login. This app makes no network request of its own."
          />
          <Fact
            term="Credentials"
            value="None. This app stores no API key and never reads Claude Code's credentials."
          />
          <Fact
            term="Tools"
            value="File and command tools are disabled, so a reply cannot read your files or run anything."
          />
          <Fact term="Telemetry" value="None. No analytics, no crash reporting, no update pings." />
          <Fact term="Logs" value="Diagnostics only — message content is never logged." />
        </dl>
      </section>

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
          description="One JSON file with every conversation, message, project and setting."
          control={
            <Button variant="secondary" size="sm" onClick={() => void exportEverything()}>
              <Download size={13} /> Export data
            </Button>
          }
        />

        <Row
          label="Delete all conversations"
          description="Removes every conversation, message and attachment from this computer. Your settings are kept."
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
