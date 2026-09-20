import { useCallback, useEffect, useState } from 'react';
import { CheckCircle2, ExternalLink, Loader2, ShieldCheck, Terminal, XCircle } from 'lucide-react';
import * as api from '@/services/api';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { Button } from '@/components/ui/Button';
import { Switch } from '@/components/ui/Switch';
import { openExternal } from '@/lib/external';
import type { ClaudeCodeStatus } from '@/types';

/**
 * First-run screen.
 *
 * There is nothing to paste and no account to connect here: the app talks to
 * Claude through the Claude Code CLI, under the login you already have. All
 * this screen does is confirm the CLI is present.
 */
export function Welcome({ onDone }: { onDone: () => void }) {
  const { refreshClaudeCode, refreshModels, set, settings } = useSettingsStore();
  const [status, setStatus] = useState<ClaudeCodeStatus | null>(null);
  const [checking, setChecking] = useState(true);
  const [autoTitle, setAutoTitle] = useState(settings?.['claude.autoTitle'] ?? true);

  const check = useCallback(async () => {
    setChecking(true);
    try {
      setStatus(await api.claudeCodeStatus());
    } catch {
      setStatus({ installed: false, version: null, path: null });
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    void check();
  }, [check]);

  async function start() {
    await set('claude.autoTitle', autoTitle);
    await set('ui.onboarded', true);
    await refreshClaudeCode();
    await refreshModels(true);
    onDone();
  }

  return (
    <div className="flex h-full items-center justify-center overflow-y-auto scroll-thin bg-canvas p-6">
      <div className="w-full max-w-lg py-8">
        <div className="mb-8 text-center">
          <div className="mx-auto mb-4 flex size-12 items-center justify-center rounded-xl bg-accent-soft">
            <Terminal size={22} className="text-accent" aria-hidden />
          </div>
          <h1 className="text-[22px] font-semibold tracking-[-0.02em] text-ink">
            Welcome to OpenClaude Desktop
          </h1>
          <p className="mx-auto mt-2 max-w-sm text-[14px] leading-relaxed text-ink-soft">
            An unofficial, open-source Claude client for Linux. It talks to Claude
            through Claude Code, using the login you already have.
          </p>
        </div>

        <div className="space-y-5 rounded-xl border border-line bg-surface p-5 shadow-subtle">
          {checking ? (
            <div className="flex items-center gap-2 py-2 text-[13.5px] text-ink-soft">
              <Loader2 size={15} className="animate-spin" aria-hidden />
              Looking for Claude Code…
            </div>
          ) : status?.installed ? (
            <div className="flex items-start gap-2.5 rounded-lg border border-success/30 bg-success/5 px-3 py-2.5">
              <CheckCircle2 size={16} className="mt-0.5 shrink-0 text-success" aria-hidden />
              <div className="min-w-0">
                <p className="text-[13.5px] text-ink">Claude Code found.</p>
                <p className="truncate font-mono text-[11.5px] text-ink-faint" title={status.path ?? ''}>
                  {status.version}
                  {status.path && ` · ${status.path}`}
                </p>
              </div>
            </div>
          ) : (
            <div className="space-y-2 rounded-lg border border-warn/30 bg-warn/5 px-3 py-2.5">
              <p className="flex items-center gap-2 text-[13.5px] text-ink">
                <XCircle size={15} className="shrink-0 text-warn" aria-hidden />
                Claude Code was not found on your PATH.
              </p>
              <p className="text-[12.5px] leading-snug text-ink-soft">
                OpenClaude has no API credentials of its own — it needs the{' '}
                <code className="font-mono text-[11.5px]">claude</code> command. Install it,
                sign in once with <code className="font-mono text-[11.5px]">claude</code>, then
                check again.
              </p>
              <button
                type="button"
                onClick={() => void openExternal('https://code.claude.com/docs')}
                className="inline-flex items-center gap-1 text-[12.5px] text-accent hover:underline"
              >
                Claude Code install guide <ExternalLink size={10} />
              </button>
            </div>
          )}

          <div className="flex items-start gap-2.5 rounded-lg border border-line bg-sunken/60 p-3">
            <ShieldCheck size={16} className="mt-0.5 shrink-0 text-success" aria-hidden />
            <div className="space-y-1 text-[12.5px] leading-snug text-ink-soft">
              <p>
                This app stores no API key and makes no network request of its own. Every
                request goes through Claude Code, running as you.
              </p>
              <p>
                Conversations are kept on this computer. There is no telemetry. File and
                command tools are switched off, so a reply cannot read your files or run
                anything.
              </p>
            </div>
          </div>

          <div className="border-t border-line pt-1">
            <Switch
              checked={autoTitle}
              onChange={setAutoTitle}
              label="Name conversations automatically"
              description="After your first message, asks Claude for a short title in a separate throwaway session. Turning this off uses the first line of your message instead."
            />
          </div>

          <div className="flex items-center gap-2 pt-1">
            <Button variant="secondary" onClick={() => void check()} disabled={checking}>
              Check again
            </Button>
            <Button
              variant="primary"
              className="flex-1"
              onClick={() => void start()}
              disabled={!status?.installed}
            >
              Start using OpenClaude
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
          OpenClaude Desktop is an independent open-source project. It is not affiliated
          with, endorsed by, or sponsored by Anthropic.
        </p>
      </div>
    </div>
  );
}
