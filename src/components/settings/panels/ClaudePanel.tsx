import { useCallback, useEffect, useState } from 'react';
import { CheckCircle2, ExternalLink, RefreshCw, Terminal, XCircle } from 'lucide-react';
import * as api from '@/services/api';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { Button } from '@/components/ui/Button';
import { Select } from '@/components/ui/Select';
import { Switch } from '@/components/ui/Switch';
import { Group, Row } from '../SettingsDialog';
import { openExternal } from '@/lib/external';
import type { ClaudeCodeStatus, EffortLevel } from '@/types';

export function ClaudePanel() {
  const { settings, set, models, modelsStale, currentModel, cliEffort, refreshModels } =
    useSettingsStore();
  const slashCommands = settings?.['claude.slashCommands'] ?? [];
  const [status, setStatus] = useState<ClaudeCodeStatus | null>(null);
  const [checking, setChecking] = useState(false);

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

  if (!settings) return null;

  return (
    <>
      <Group
        title="Connection"
        description="OpenClaude has no API credentials of its own. Every request runs through the Claude Code CLI, as you, using the login you already established with it."
      >
        <div className="py-2">
          {status?.installed ? (
            <div className="flex items-start gap-2.5 rounded-lg border border-line bg-sunken/60 px-3 py-2.5">
              <CheckCircle2 size={16} className="mt-0.5 shrink-0 text-success" aria-hidden />
              <div className="min-w-0 flex-1">
                <p className="text-[13.5px] text-ink">Connected through Claude Code.</p>
                <p className="truncate font-mono text-[11.5px] text-ink-faint" title={status.path ?? ''}>
                  {status.version}
                  {status.path && ` · ${status.path}`}
                </p>
              </div>
              <Button size="sm" variant="ghost" onClick={() => void check()} loading={checking}>
                <RefreshCw size={13} /> Recheck
              </Button>
            </div>
          ) : (
            <div className="space-y-2 rounded-lg border border-warn/30 bg-warn/5 px-3 py-2.5">
              <p className="flex items-center gap-2 text-[13.5px] text-ink">
                <XCircle size={15} className="shrink-0 text-warn" aria-hidden />
                Claude Code was not found on your PATH.
              </p>
              <p className="text-[12.5px] leading-snug text-ink-soft">
                Install the <code className="font-mono text-[11.5px]">claude</code> command and
                sign in with it once, then recheck.
              </p>
              <div className="flex items-center gap-2 pt-0.5">
                <Button size="sm" variant="secondary" onClick={() => void check()} loading={checking}>
                  <RefreshCw size={13} /> Recheck
                </Button>
                <button
                  type="button"
                  onClick={() => void openExternal('https://code.claude.com/docs')}
                  className="inline-flex items-center gap-1 text-[12.5px] text-accent hover:underline"
                >
                  Install guide <ExternalLink size={10} />
                </button>
              </div>
            </div>
          )}

          <p className="mt-2.5 flex items-start gap-1.5 text-[12px] leading-snug text-ink-faint">
            <Terminal size={12} className="mt-0.5 shrink-0" aria-hidden />
            File and command tools are disabled for these conversations, so a reply cannot
            read your files or run anything.
          </p>
        </div>
      </Group>

      <Group
        title="Model"
        description={
          currentModel
            ? `Claude Code reports it is currently using ${currentModel}${
                cliEffort ? ` at ${cliEffort} effort` : ''
              }.`
            : 'Asked of Claude Code directly, so the list matches your installation.'
        }
      >
        <Row
          label="Default model"
          description={
            modelsStale
              ? 'Claude Code could not be asked, so this is a fallback list.'
              : `${models.length} aliases available. Each follows whatever model is current.`
          }
          control={
            <Select
              value={settings['claude.defaultModel'] ?? ''}
              onChange={(e) => void set('claude.defaultModel', e.target.value)}
              aria-label="Default model"
            >
              {models.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.displayName}
                </option>
              ))}
            </Select>
          }
        />
        <Row
          label="Refresh"
          description="Re-asks Claude Code which models it accepts. Costs nothing."
          control={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void refreshModels(true)}
            >
              <RefreshCw size={13} /> Refresh models
            </Button>
          }
        />
      </Group>

      <Group title="Reasoning">
        <Row
          label="Effort"
          description="How hard Claude works before answering. Higher is more thorough and slower; leave on the default unless you have a reason."
          control={
            <Select
              value={settings['claude.effort'] ?? ''}
              onChange={(e) =>
                void set('claude.effort', (e.target.value || null) as EffortLevel | null)
              }
              aria-label="Effort"
            >
              <option value="">
                {cliEffort ? `Claude Code default (${cliEffort})` : 'Claude Code default'}
              </option>
              <option value="low">Low</option>
              <option value="medium">Medium</option>
              <option value="high">High</option>
              <option value="xhigh">Very high</option>
              <option value="max">Maximum</option>
            </Select>
          }
        />
      </Group>

      <Group
        title="Slash commands"
        description="Type / in the composer to use Claude Code's commands and skills. The list is whatever your installation offers, picked up automatically from your first message."
      >
        <Row
          label="Known commands"
          description={
            slashCommands.length > 0
              ? slashCommands.slice(0, 8).join(', ') +
                (slashCommands.length > 8 ? `, and ${slashCommands.length - 8} more` : '')
              : 'None discovered yet — send a message and they will appear.'
          }
          control={
            <span className="block text-right text-[13px] tabular-nums text-ink">
              {slashCommands.length}
            </span>
          }
        />
      </Group>

      <Group title="Conversation titles">
        <Switch
          checked={settings['claude.autoTitle']}
          onChange={(v) => void set('claude.autoTitle', v)}
          label="Name conversations automatically"
          description="Asks Claude for a short title in a separate throwaway session after your first message, so it never appears in the transcript. With this off, the first line of your message is used instead."
        />
        <Row
          label="Model used for titles"
          description="Defaults to the conversation's own model. A smaller one is plenty for a title."
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
