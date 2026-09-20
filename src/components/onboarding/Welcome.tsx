import { useCallback, useEffect, useState } from 'react';
import { CheckCircle2, ExternalLink, KeyRound, Loader2, ShieldCheck, XCircle } from 'lucide-react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Switch } from '@/components/ui/Switch';
import { openExternal } from '@/lib/external';
import { SignInOptions } from './SignInOptions';
import type { AuthOptions } from '@/types';

type TestState =
  | { phase: 'idle' }
  | { phase: 'testing' }
  | { phase: 'ok'; modelCount: number }
  | { phase: 'failed'; message: string };

/**
 * First-run screen.
 *
 * Deliberately explicit about three things before anything is stored: where
 * the key goes, what the app sends, and that auto-titling costs a small API
 * call. Surprising someone with a charge is not acceptable, however small.
 */
export function Welcome({ onDone }: { onDone: () => void }) {
  const { refreshCredentials, refreshModels, set, settings } = useSettingsStore();
  const [key, setKey] = useState('');
  const [test, setTest] = useState<TestState>({ phase: 'idle' });
  const [saving, setSaving] = useState(false);
  const [autoTitle, setAutoTitle] = useState(settings?.['claude.autoTitle'] ?? true);
  const [backend, setBackend] = useState<'keyring' | 'memoryOnly' | null>(null);
  const [auth, setAuth] = useState<AuthOptions | null>(null);

  const refreshAuth = useCallback(async () => {
    const next = await api.authOptions().catch(() => null);
    setAuth(next);
    // Browser sign-in completing is enough to get past this screen.
    if (next?.mode === 'oauth' && next.cli.signedIn) {
      await set('ui.onboarded', true);
      await refreshCredentials();
      await refreshModels(true);
      onDone();
    }
  }, [set, refreshCredentials, refreshModels, onDone]);

  useEffect(() => {
    void refreshAuth();
  }, [refreshAuth]);

  async function runTest() {
    setTest({ phase: 'testing' });
    try {
      const result = await api.testApiKey(key);
      setTest({ phase: 'ok', modelCount: result.modelCount });
    } catch (err) {
      setTest({ phase: 'failed', message: AppError.from(err).message });
    }
  }

  async function saveAndContinue() {
    setSaving(true);
    try {
      const status = await api.setApiKey(key);
      setBackend(status.backend);
      await set('claude.autoTitle', autoTitle);
      await set('ui.onboarded', true);
      await refreshCredentials();
      await refreshModels(true);
      onDone();
    } catch (err) {
      setTest({ phase: 'failed', message: AppError.from(err).message });
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex h-full items-center justify-center overflow-y-auto scroll-thin bg-canvas p-6">
      <div className="w-full max-w-lg py-8">
        <div className="mb-8 text-center">
          <div className="mx-auto mb-4 flex size-12 items-center justify-center rounded-xl bg-accent-soft">
            <KeyRound size={22} className="text-accent" aria-hidden />
          </div>
          <h1 className="text-[22px] font-semibold tracking-[-0.02em] text-ink">
            Welcome to OpenClaude Desktop
          </h1>
          <p className="mx-auto mt-2 max-w-sm text-[14px] leading-relaxed text-ink-soft">
            An unofficial, open-source Claude client for Linux. Connect your own Anthropic API
            key to get started.
          </p>
        </div>

        <div className="space-y-5 rounded-xl border border-line bg-surface p-5 shadow-subtle">
          <SignInOptions
            options={auth}
            onChanged={refreshAuth}
            onChooseApiKey={() => document.getElementById('apikey')?.focus()}
          />

          <div className="relative py-0.5 text-center">
            <span className="relative z-10 bg-surface px-2 text-[11.5px] uppercase tracking-wide text-ink-faint">
              or
            </span>
            <span className="absolute left-0 right-0 top-1/2 h-px bg-line" aria-hidden />
          </div>

          <div className="space-y-1.5">
            <label htmlFor="apikey" className="block text-[13px] font-medium text-ink">
              Anthropic API key
            </label>
            <Input
              id="apikey"
              data-autofocus
              type="password"
              value={key}
              onChange={(e) => {
                setKey(e.target.value);
                setTest({ phase: 'idle' });
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && key.trim()) void runTest();
              }}
              placeholder="sk-ant-…"
              autoComplete="off"
              spellCheck={false}
              className="font-mono text-[13px]"
            />
            <p className="flex items-center gap-1 text-[12px] text-ink-faint">
              Get one from
              <button
                type="button"
                onClick={() => void openExternal('https://console.anthropic.com/settings/keys')}
                className="inline-flex items-center gap-0.5 text-accent hover:underline"
              >
                console.anthropic.com <ExternalLink size={10} />
              </button>
            </p>
          </div>

          <div className="flex items-start gap-2.5 rounded-lg border border-line bg-sunken/60 p-3">
            <ShieldCheck size={16} className="mt-0.5 shrink-0 text-success" aria-hidden />
            <div className="space-y-1 text-[12.5px] leading-snug text-ink-soft">
              <p>
                Your key is stored in your system keyring (GNOME Keyring / KWallet) — never in
                plain text, never in the app's database, and never in a log.
              </p>
              <p>
                It is sent only to <code className="font-mono text-[11.5px]">api.anthropic.com</code>.
                Conversations are kept on this computer. There is no telemetry.
              </p>
            </div>
          </div>

          <div className="border-t border-line pt-1">
            <Switch
              checked={autoTitle}
              onChange={setAutoTitle}
              label="Name conversations automatically"
              description="After your first message, makes one short request to a small model to generate a title. Turning this off uses the first line of your message instead, at no cost."
            />
          </div>

          {test.phase === 'ok' && (
            <div className="flex items-center gap-2 rounded-lg border border-success/30 bg-success/5 px-3 py-2 text-[13px] text-ink">
              <CheckCircle2 size={15} className="shrink-0 text-success" aria-hidden />
              Connected. {test.modelCount} model{test.modelCount === 1 ? '' : 's'} available.
            </div>
          )}
          {test.phase === 'failed' && (
            <div className="flex items-start gap-2 rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-[13px] text-ink">
              <XCircle size={15} className="mt-0.5 shrink-0 text-danger" aria-hidden />
              <span>{test.message}</span>
            </div>
          )}
          {backend === 'memoryOnly' && (
            <p className="text-[12.5px] text-warn">
              No system keyring was reachable, so the key is held only for this session. You will
              be asked for it again next launch.
            </p>
          )}

          <div className="flex items-center gap-2 pt-1">
            <Button
              variant="secondary"
              onClick={() => void runTest()}
              disabled={!key.trim() || test.phase === 'testing'}
            >
              {test.phase === 'testing' && <Loader2 size={14} className="animate-spin" />}
              Test connection
            </Button>
            <Button
              variant="primary"
              className="flex-1"
              onClick={() => void saveAndContinue()}
              disabled={!key.trim()}
              loading={saving}
            >
              Save and continue
            </Button>
          </div>

          <button
            type="button"
            onClick={() => void set('ui.onboarded', true).then(onDone)}
            className="w-full text-center text-[12.5px] text-ink-faint hover:text-ink-soft hover:underline"
          >
            Skip for now — browse the app without connecting
          </button>
        </div>

        <p className="mt-6 text-center text-[11.5px] leading-relaxed text-ink-faint">
          OpenClaude Desktop is an independent open-source project. It is not affiliated with,
          endorsed by, or sponsored by Anthropic.
        </p>
      </div>
    </div>
  );
}
