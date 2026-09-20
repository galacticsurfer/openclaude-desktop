import { useEffect, useState } from 'react';
import { CheckCircle2, Copy, RefreshCw, Save, Sparkles } from 'lucide-react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Switch } from '@/components/ui/Switch';
import { Group, Row } from '../SettingsDialog';
import appIcon from '@/assets/openclaude.png';
import type { AppInfo } from '@/types';

export function AdvancedPanel() {
  const { settings, set } = useSettingsStore();
  const toast = useUIStore((s) => s.toast);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [integrity, setIntegrity] = useState<string | null>(null);

  useEffect(() => {
    void api.appInfo().then(setInfo).catch(() => setInfo(null));
  }, []);

  if (!settings) return null;

  async function backup() {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const path = await save({
        defaultPath: `openclaude-backup-${new Date().toISOString().slice(0, 10)}.db`,
      });
      if (!path) return;
      await api.backupDatabase(path);
      toast('success', 'Backup written.');
    } catch (err) {
      toast('error', AppError.from(err).message);
    }
  }

  return (
    <>
      <Group title="Database">
        <Row
          label="Back up now"
          description="Writes a consistent copy of the database using SQLite's online backup — safe to run while the app is in use."
          control={
            <Button variant="secondary" size="sm" onClick={() => void backup()}>
              <Save size={13} /> Back up…
            </Button>
          }
        />
        <Row
          label="Check integrity"
          description="Runs PRAGMA integrity_check against the local database."
          control={
            <div className="space-y-1.5">
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  void api
                    .storageStats()
                    .then((s) => setIntegrity(s.integrity))
                    .catch((e) => toast('error', AppError.from(e).message));
                }}
              >
                <CheckCircle2 size={13} /> Check
              </Button>
              {integrity && (
                <p className={`text-[12px] ${integrity === 'ok' ? 'text-success' : 'text-danger'}`}>
                  {integrity === 'ok' ? 'No problems found.' : integrity}
                </p>
              )}
            </div>
          }
        />
        <Row
          label="Compact database"
          description="Reclaims space after deleting a lot of history."
          control={
            <Button
              variant="secondary"
              size="sm"
              onClick={() => {
                void api
                  .vacuumDatabase()
                  .then(() => toast('success', 'Database compacted.'))
                  .catch((e) => toast('error', AppError.from(e).message));
              }}
            >
              <Sparkles size={13} /> Compact
            </Button>
          }
        />
        <Row
          label="Rebuild search index"
          description="Use if search stops returning results it should."
          control={
            <Button
              variant="secondary"
              size="sm"
              onClick={() => {
                void api
                  .rebuildSearchIndex()
                  .then(() => toast('success', 'Search index rebuilt.'))
                  .catch((e) => toast('error', AppError.from(e).message));
              }}
            >
              <RefreshCw size={13} /> Rebuild
            </Button>
          }
        />
        <Row
          label="Remove orphaned attachments"
          description="Deletes attachment files on disk that no conversation references any more."
          control={
            <Button
              variant="secondary"
              size="sm"
              onClick={() => {
                void api
                  .pruneOrphanAttachments()
                  .then((n) => toast('success', n === 0 ? 'Nothing to remove.' : `Removed ${n} file(s).`))
                  .catch((e) => toast('error', AppError.from(e).message));
              }}
            >
              Prune
            </Button>
          }
        />
      </Group>

      <Group title="Retention">
        <Row
          label="Keep trashed conversations for"
          description="Conversations in the trash are deleted permanently after this many days. Set 0 to keep them indefinitely."
          control={
            <Input
              type="number"
              min={0}
              max={3650}
              value={settings['advanced.trashRetentionDays']}
              onChange={(e) => void set('advanced.trashRetentionDays', Number(e.target.value))}
              aria-label="Trash retention in days"
            />
          }
        />
      </Group>

      <Group title="Diagnostics">
        <Switch
          checked={settings['advanced.debugLogs']}
          onChange={(v) => void set('advanced.debugLogs', v)}
          label="Verbose logging"
          description="More detail in the log file. Message content is never logged regardless of this setting. Takes effect on restart."
        />
      </Group>

      {info && (
        <Group title="About">
          <div className="flex items-center gap-3 pb-3 pt-1">
            <img
              src={appIcon}
              alt=""
              width={40}
              height={40}
              className="size-10 rounded-lg shadow-subtle"
            />
            <div className="min-w-0">
              <p className="text-[14px] font-medium text-ink">OpenClaude Desktop</p>
              <p className="text-[12px] text-ink-faint">
                Version {info.version} · unofficial, open source
              </p>
            </div>
          </div>
          <dl className="space-y-1.5 py-2 text-[12.5px]">
            <PathRow label="Version" value={info.version} />
            <PathRow label="Schema" value={`v${info.schemaVersion}`} />
            <PathRow label="Database" value={info.databasePath} copyable />
            <PathRow label="Data" value={info.dataDir} copyable />
            <PathRow label="Config" value={info.configDir} copyable />
            <PathRow label="Logs" value={info.logDir} copyable />
            {info.devMode && <PathRow label="Mode" value="developer (OPENCLAUDE_DEV=1)" />}
          </dl>
        </Group>
      )}
    </>
  );
}

function PathRow({ label, value, copyable }: { label: string; value: string; copyable?: boolean }) {
  const toast = useUIStore((s) => s.toast);
  return (
    <div className="flex items-center gap-2">
      <dt className="w-20 shrink-0 text-ink-faint">{label}</dt>
      <dd className="min-w-0 flex-1 truncate font-mono text-ink-soft" title={value}>
        {value}
      </dd>
      {copyable && (
        <button
          type="button"
          aria-label={`Copy ${label} path`}
          onClick={() => {
            void import('@tauri-apps/plugin-clipboard-manager').then(({ writeText }) => {
              void writeText(value).then(() => toast('info', 'Path copied.'));
            });
          }}
          className="shrink-0 rounded p-1 text-ink-faint hover:text-ink"
        >
          <Copy size={12} />
        </button>
      )}
    </div>
  );
}
