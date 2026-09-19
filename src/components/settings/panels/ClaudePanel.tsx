import { useState } from 'react';
import { CheckCircle2, ExternalLink, KeyRound, ShieldAlert, ShieldCheck, Trash2, XCircle } from 'lucide-react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Switch } from '@/components/ui/Switch';
import { Group, Row } from '../SettingsDialog';
import { openExternal } from '@/lib/external';

export function ClaudePanel() {
  const { settings, set, credentials, refreshCredentials, models, refreshModels, modelsStale } =
    useSettingsStore();
  const { toast, confirm } = useUIStore();
  const [newKey, setNewKey] = useState('');
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);

  if (!settings) return null;

  async function saveKey() {
    try {
      await api.setApiKey(newKey);
      setNewKey('');
      setResult({ ok: true, message: 'Key saved.' });
      await refreshCredentials();
      await refreshModels(true);
    } catch (err) {
      setResult({ ok: false, message: AppError.from(err).message });
    }
  }

  async function test() {
    setTesting(true);
    try {
      const r = await api.testApiKey(newKey.trim() || undefined);
      setResult({ ok: true, message: `Connected. ${r.modelCount} models available.` });
    } catch (err) {
      setResult({ ok: false, message: AppError.from(err).message });
    } finally {
      setTesting(false);
    }
  }

  return (
    <>
      <Group title="API key">
        <div className="space-y-3 py-2">
          {credentials?.configured ? (
            <div className="flex items-center gap-2.5 rounded-lg border border-line bg-sunken/60 px-3 py-2.5">
              {credentials.backend === 'keyring' ? (
                <ShieldCheck size={16} className="shrink-0 text-success" aria-hidden />
              ) : (
                <ShieldAlert size={16} className="shrink-0 text-warn" aria-hidden />
              )}
              <div className="min-w-0 flex-1">
                <p className="text-[13.5px] text-ink">
                  Key ending <code className="font-mono">…{credentials.hint}</code> is configured.
                </p>
                <p className="text-[12px] text-ink-faint">
                  {credentials.backend === 'keyring'
                    ? 'Stored in your system keyring.'
                    : 'No system keyring was reachable — held in memory for this session only.'}
                </p>
              </div>
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  confirm({
                    title: 'Remove the API key?',
                    body: 'OpenClaude will not be able to reach Claude until you add a key again. Your conversations are not affected.',
                    confirmLabel: 'Remove key',
                    destructive: true,
                    onConfirm: async () => {
                      await api.deleteApiKey();
                      await refreshCredentials();
                      toast('info', 'API key removed.');
                    },
                  })
                }
              >
                <Trash2 size={13} /> Remove
              </Button>
            </div>
          ) : (
            <div className="flex items-center gap-2.5 rounded-lg border border-warn/30 bg-warn/5 px-3 py-2.5 text-[13.5px] text-ink">
              <KeyRound size={16} className="shrink-0 text-warn" aria-hidden />
              No API key configured.
            </div>
          )}

          <div className="flex gap-2">
            <Input
              type="password"
              value={newKey}
              onChange={(e) => {
                setNewKey(e.target.value);
                setResult(null);
              }}
              placeholder={credentials?.configured ? 'Replace with a new key…' : 'sk-ant-…'}
              autoComplete="off"
              spellCheck={false}
              aria-label="Anthropic API key"
              className="font-mono text-[13px]"
            />
            <Button variant="secondary" onClick={() => void test()} loading={testing} disabled={testing}>
              Test
            </Button>
            <Button variant="primary" onClick={() => void saveKey()} disabled={!newKey.trim()}>
              Save
            </Button>
          </div>

          {result && (
            <p
              className={`flex items-center gap-1.5 text-[12.5px] ${result.ok ? 'text-success' : 'text-danger'}`}
            >
              {result.ok ? <CheckCircle2 size={13} /> : <XCircle size={13} />}
              {result.message}
            </p>
          )}

          <p className="text-[12px] text-ink-faint">
            Keys are issued at{' '}
            <button
              type="button"
              onClick={() => void openExternal('https://console.anthropic.com/settings/keys')}
              className="inline-flex items-center gap-0.5 text-accent hover:underline"
            >
              console.anthropic.com <ExternalLink size={9} />
            </button>
            . The key never leaves this machine except in requests to Anthropic.
          </p>
        </div>
      </Group>

      <Group title="Models">
        <Row
          label="Default model"
          description={
            modelsStale
              ? 'This list could not be refreshed from the API and may be incomplete.'
              : 'Used for new conversations.'
          }
          control={
            <Select
              value={settings['claude.defaultModel'] ?? ''}
              onChange={(e) => void set('claude.defaultModel', e.target.value)}
              aria-label="Default model"
            >
              {models.length === 0 && <option value="">Loading…</option>}
              {models.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.displayName}
                </option>
              ))}
            </Select>
          }
        />
        <Row
          label="Maximum response length"
          description="Upper bound on tokens in a single reply. Higher values allow longer answers and cost more."
          control={
            <Input
              type="number"
              min={256}
              max={64000}
              step={256}
              value={settings['claude.maxTokens']}
              onChange={(e) => void set('claude.maxTokens', Number(e.target.value))}
              aria-label="Maximum response tokens"
            />
          }
        />
        <Row
          label="Temperature"
          description="Leave blank to use the model's default. 0 is the most deterministic."
          control={
            <Input
              type="number"
              min={0}
              max={1}
              step={0.1}
              value={settings['claude.temperature'] ?? ''}
              placeholder="default"
              onChange={(e) =>
                void set('claude.temperature', e.target.value === '' ? null : Number(e.target.value))
              }
              aria-label="Temperature"
            />
          }
        />
      </Group>

      <Group title="Conversation titles">
        <Switch
          checked={settings['claude.autoTitle']}
          onChange={(v) => void set('claude.autoTitle', v)}
          label="Name conversations automatically"
          description="Makes one short request to a small model after your first message. With this off, the first line of your message is used instead — instant and free."
        />
        <Row
          label="Model used for titles"
          description="Defaults to the conversation's own model. Pick the cheapest model you have access to."
          control={
            <Select
              value={settings['claude.titleModel'] ?? ''}
              onChange={(e) => void set('claude.titleModel', e.target.value || null)}
              disabled={!settings['claude.autoTitle']}
              aria-label="Title model"
            >
              <option value="">Same as conversation</option>
              {models.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.displayName}
                </option>
              ))}
            </Select>
          }
        />
      </Group>
    </>
  );
}
