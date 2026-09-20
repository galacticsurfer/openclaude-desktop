import { useCallback, useEffect, useState } from 'react';
import { CheckCircle2, ExternalLink, Globe, KeyRound, Loader2, RefreshCw } from 'lucide-react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { Button } from '@/components/ui/Button';
import { openExternal } from '@/lib/external';
import { cn } from '@/lib/cn';
import type { AuthOptions } from '@/types';

interface Props {
  options: AuthOptions | null;
  onChanged: () => void | Promise<void>;
  /** Called when the user picks the API-key route, so the parent can focus it. */
  onChooseApiKey?: () => void;
}

/**
 * Browser sign-in, offered alongside the API key.
 *
 * Deliberately *not* Claude Code's `/login`: that is a first-party OAuth
 * client, and a third-party app can only join it by impersonating that client
 * or reading its credentials file. This uses the Anthropic CLI's own OAuth
 * (`ant auth login`) instead, which is a documented, supported flow whose
 * profile the SDKs already share.
 */
export function SignInOptions({ options, onChanged, onChooseApiKey }: Props) {
  const [busy, setBusy] = useState(false);
  const [waiting, setWaiting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const cli = options?.cli;
  const signedIn = cli?.signedIn ?? false;

  // After launching the browser flow, poll until the CLI reports a usable
  // profile — the sign-in happens outside this window, so there is nothing
  // to await.
  useEffect(() => {
    if (!waiting) return;
    const started = Date.now();
    const timer = window.setInterval(() => {
      void (async () => {
        await onChanged();
        const fresh = await api.authOptions().catch(() => null);
        if (fresh?.cli.signedIn) {
          setWaiting(false);
        } else if (Date.now() - started > 5 * 60_000) {
          setWaiting(false);
          setError('Sign-in timed out. Try again, or use an API key.');
        }
      })();
    }, 2000);
    return () => window.clearInterval(timer);
  }, [waiting, onChanged]);

  const signIn = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      await api.oauthBeginLogin(options?.profile ?? null);
      setWaiting(true);
    } catch (err) {
      setError(AppError.from(err).message);
    } finally {
      setBusy(false);
    }
  }, [options?.profile]);

  const use = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      await api.setAuthMode('oauth');
      await onChanged();
    } catch (err) {
      setError(AppError.from(err).message);
    } finally {
      setBusy(false);
    }
  }, [onChanged]);

  // Without the CLI there is only one route; explain the other rather than
  // showing a button that cannot work.
  if (!cli?.available) {
    return (
      <div className="rounded-lg border border-line bg-sunken/50 p-3">
        <p className="flex items-center gap-1.5 text-[13px] font-medium text-ink">
          <Globe size={14} className="text-ink-faint" aria-hidden />
          Prefer signing in with a browser?
        </p>
        <p className="mt-1 text-[12.5px] leading-snug text-ink-soft">
          Install the{' '}
          <button
            type="button"
            onClick={() => void openExternal('https://github.com/anthropics/anthropic-cli')}
            className="inline-flex items-center gap-0.5 text-accent hover:underline"
          >
            Anthropic CLI <ExternalLink size={9} />
          </button>{' '}
          and this screen will offer <code className="font-mono text-[11.5px]">ant auth login</code>{' '}
          — browser sign-in with no key to paste. It still bills your API
          account; it is not a Claude Pro or Max subscription.
        </p>
      </div>
    );
  }

  const active = options?.mode === 'oauth';

  return (
    <div
      className={cn(
        'rounded-lg border p-3 transition-colors',
        active ? 'border-accent/40 bg-accent-soft/40' : 'border-line bg-sunken/50',
      )}
    >
      <div className="flex items-start gap-2.5">
        <Globe size={16} className="mt-0.5 shrink-0 text-accent" aria-hidden />
        <div className="min-w-0 flex-1">
          <p className="text-[13.5px] font-medium text-ink">Sign in with your browser</p>
          <p className="mt-0.5 text-[12.5px] leading-snug text-ink-soft">
            Uses the Anthropic CLI's sign-in. No key to paste, and the token is
            short-lived and refreshed for you. Bills your API account the same
            way a key does — this is not a Pro or Max subscription.
          </p>

          {cli.detail && signedIn && (
            <p className="mt-1.5 truncate font-mono text-[11.5px] text-ink-faint" title={cli.detail}>
              {cli.detail.split('\n')[0]}
            </p>
          )}

          {error && <p className="mt-1.5 text-[12.5px] text-danger">{error}</p>}

          <div className="mt-2.5 flex flex-wrap items-center gap-2">
            {signedIn ? (
              <>
                <span className="flex items-center gap-1 text-[12.5px] text-success">
                  <CheckCircle2 size={13} /> Signed in
                </span>
                {!active && (
                  <Button size="sm" variant="primary" onClick={() => void use()} loading={busy}>
                    Use this sign-in
                  </Button>
                )}
                {active && (
                  <span className="text-[12.5px] text-ink-faint">Currently in use.</span>
                )}
                <Button size="sm" variant="ghost" onClick={() => void signIn()} disabled={busy}>
                  <RefreshCw size={13} /> Sign in again
                </Button>
              </>
            ) : (
              <>
                <Button size="sm" variant="secondary" onClick={() => void signIn()} loading={busy || waiting}>
                  {waiting ? <Loader2 size={13} className="animate-spin" /> : <Globe size={13} />}
                  {waiting ? 'Waiting for your browser…' : 'Sign in with browser'}
                </Button>
                {onChooseApiKey && (
                  <button
                    type="button"
                    onClick={onChooseApiKey}
                    className="inline-flex items-center gap-1 text-[12.5px] text-ink-faint hover:text-ink-soft hover:underline"
                  >
                    <KeyRound size={12} /> use an API key instead
                  </button>
                )}
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
