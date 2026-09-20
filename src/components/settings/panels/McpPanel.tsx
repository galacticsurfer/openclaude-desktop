import { useEffect, useState } from 'react';
import { ChevronDown, ChevronRight, Plug, Plus, RefreshCw, Trash2 } from 'lucide-react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useUIStore } from '@/stores/useUIStore';
import { Button } from '@/components/ui/Button';
import { IconButton } from '@/components/ui/IconButton';
import { Switch } from '@/components/ui/Switch';
import { Select } from '@/components/ui/Select';
import { Spinner } from '@/components/ui/Spinner';
import { openExternal } from '@/lib/external';
import { useConversationStore } from '@/stores/useConversationStore';
import type { McpServer, ToolPermission } from '@/types';

/**
 * MCP servers and the tools they are allowed to run.
 *
 * Permission is granted *before* a tool can run, not while it runs: a tool
 * with no explicit Allow is never passed to the CLI, and anything that would
 * have raised a prompt is denied outright — there is no terminal behind this
 * window to answer one. That makes approval a deliberate act here rather
 * than a reflexive click in the middle of a reply.
 */
export function McpPanel() {
  const toast = useUIStore((s) => s.toast);
  const confirm = useUIStore((s) => s.confirm);

  const [servers, setServers] = useState<McpServer[]>([]);
  const [perms, setPerms] = useState<ToolPermission[]>([]);
  const [tools, setTools] = useState<Record<string, string[]>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const projects = useConversationStore((st) => st.projects);
  const projectName = (id: string) => projects.find((p) => p.id === id)?.name;

  async function refresh() {
    try {
      const [s, p] = await Promise.all([api.listMcpServers(), api.listMcpPermissions()]);
      setServers(s);
      setPerms(p);
    } catch (err) {
      toast('error', AppError.from(err).message);
    }
  }

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const decisionFor = (serverId: string, tool: string) =>
    perms.find((p) => p.serverId === serverId && p.toolName === tool)?.decision ?? 'ask';

  async function discover(server: McpServer) {
    setBusy(server.id);
    try {
      const found = await api.discoverMcpTools(server.id);
      setTools((t) => ({ ...t, [server.id]: found }));
    } catch (err) {
      toast('error', AppError.from(err).message);
    } finally {
      setBusy(null);
    }
  }

  async function decide(server: McpServer, tool: string, decision: 'allow' | 'deny' | 'ask') {
    try {
      await api.decideMcpTool(server.id, tool, 'tool', decision);
      await refresh();
    } catch (err) {
      toast('error', AppError.from(err).message);
    }
  }

  const allowedCount = (serverId: string) =>
    perms.filter((p) => p.serverId === serverId && p.decision === 'allow').length;

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-warn/40 bg-warn-soft/30 p-3">
        <p className="text-[12.5px] leading-relaxed text-ink-soft">
          <strong className="font-medium text-ink">Tools can act on your behalf.</strong> An MCP
          server runs as you, with your access. Claude decides when to call a tool based on the
          conversation — including text you paste in — so only enable servers you trust, and
          approve only the tools you actually need. Nothing runs until you allow it here.
        </p>
      </div>

      {servers.length === 0 && !adding && (
        <div className="rounded-lg border border-line bg-sunken/40 p-5 text-center">
          <span className="mx-auto mb-2 flex size-9 items-center justify-center rounded-lg bg-accent-soft">
            <Plug size={17} className="text-accent" aria-hidden />
          </span>
          <p className="text-[13.5px] text-ink">No MCP servers configured</p>
          <p className="mx-auto mt-1 max-w-prose text-[12.5px] text-ink-faint">
            Add one to let Claude use tools you grant it — reading a folder, querying a database,
            working with an issue tracker.
          </p>
        </div>
      )}

      <ul className="space-y-2">
        {servers.map((s) => (
          <li key={s.id} className="rounded-lg border border-line">
            <div className="flex items-center gap-2 px-3 py-2">
              <button
                type="button"
                onClick={() => setExpanded((e) => (e === s.id ? null : s.id))}
                aria-expanded={expanded === s.id}
                className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
              >
                {expanded === s.id ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                <span className="truncate text-[13.5px] font-medium text-ink">{s.name}</span>
                <span className="font-mono text-[11px] text-ink-faint">{s.transport}</span>
                <span className="text-[11px] text-ink-faint">
                  {s.projectId === null
                    ? 'all conversations'
                    : (projectName(s.projectId) ?? 'one project')}
                </span>
                {allowedCount(s.id) > 0 && (
                  <span className="rounded bg-accent-soft px-1.5 py-px text-[11px] text-accent">
                    {allowedCount(s.id)} allowed
                  </span>
                )}
              </button>
              <Switch
                checked={s.enabled}
                onChange={(v) => {
                  void api
                    .setMcpServerEnabled(s.id, v)
                    .then(refresh)
                    .catch((e: unknown) => toast('error', AppError.from(e).message));
                }}
                label={`Enable ${s.name}`}
              />
              <IconButton
                label={`Remove ${s.name}`}
                size="sm"
                onClick={() =>
                  confirm({
                    title: `Remove "${s.name}"?`,
                    body: 'Its tool permissions are removed with it.',
                    confirmLabel: 'Remove',
                    destructive: true,
                    onConfirm: () =>
                      void api.deleteMcpServer(s.id).then(refresh),
                  })
                }
              >
                <Trash2 size={13} />
              </IconButton>
            </div>

            {expanded === s.id && (
              <div className="border-t border-line px-3 py-2.5">
                <div className="mb-2 flex items-center gap-2">
                  <Button size="sm" variant="secondary" onClick={() => void discover(s)}>
                    {busy === s.id ? <Spinner className="size-3.5" /> : <RefreshCw size={13} />}
                    List tools
                  </Button>
                  <span className="text-[12px] text-ink-faint">
                    Asking a server what it offers grants nothing.
                  </span>
                </div>

                {tools[s.id]?.length === 0 && (
                  <p className="text-[12.5px] text-ink-faint">
                    This server reported no tools. Check the command and that it starts cleanly.
                  </p>
                )}

                <ul className="divide-y divide-line">
                  {(tools[s.id] ?? []).map((t) => (
                    <li key={t} className="flex items-center gap-3 py-1.5">
                      <code className="min-w-0 flex-1 truncate font-mono text-[12px] text-ink-soft">
                        {t}
                      </code>
                      <div className="w-32 shrink-0">
                        <Select
                          value={decisionFor(s.id, t)}
                          aria-label={`Permission for ${t}`}
                          onChange={(e) =>
                            void decide(s, t, e.target.value as 'allow' | 'deny' | 'ask')
                          }
                        >
                          <option value="ask">Not allowed</option>
                          <option value="allow">Allow</option>
                          <option value="deny">Deny</option>
                        </Select>
                      </div>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </li>
        ))}
      </ul>

      {adding ? (
        <AddServer
          onCancel={() => setAdding(false)}
          onAdded={() => {
            setAdding(false);
            void refresh();
          }}
        />
      ) : (
        <Button variant="secondary" onClick={() => setAdding(true)}>
          <Plus size={14} /> Add a server
        </Button>
      )}

      <button
        type="button"
        onClick={() => void openExternal('https://modelcontextprotocol.io')}
        className="text-[13px] text-accent hover:underline"
      >
        About the Model Context Protocol
      </button>
    </div>
  );
}

function AddServer({ onCancel, onAdded }: { onCancel: () => void; onAdded: () => void }) {
  const toast = useUIStore((s) => s.toast);
  const projects = useConversationStore((s) => s.projects);
  const [projectId, setProjectId] = useState('');
  const [name, setName] = useState('');
  const [transport, setTransport] = useState<'stdio' | 'sse' | 'http'>('stdio');
  const [command, setCommand] = useState('');
  const [args, setArgs] = useState('');
  const [url, setUrl] = useState('');

  async function submit() {
    try {
      await api.addMcpServer({
        name: name.trim(),
        transport,
        command: command.trim(),
        // Naive split is right here: a server command with quoted spaces in
        // an argument is rare, and the field is one argument per space.
        args: args.split(/\s+/).filter(Boolean),
        env: {},
        url: url.trim() === '' ? null : url.trim(),
        projectId: projectId === '' ? null : projectId,
      });
      onAdded();
    } catch (err) {
      toast('error', AppError.from(err).message);
    }
  }

  return (
    <div className="space-y-3 rounded-lg border border-line bg-sunken/40 p-3">
      <div className="grid grid-cols-2 gap-3">
        <label className="block">
          <span className="mb-1 block text-[12.5px] text-ink-soft">Name</span>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="notes"
            className="h-8 w-full rounded border border-line bg-surface px-2 text-[13px] text-ink placeholder:text-ink-faint"
          />
        </label>
        <label className="block">
          <span className="mb-1 block text-[12.5px] text-ink-soft">Transport</span>
          <Select
            value={transport}
            aria-label="Transport"
            onChange={(e) => setTransport(e.target.value as 'stdio' | 'sse' | 'http')}
          >
            <option value="stdio">stdio (local process)</option>
            <option value="http">http</option>
            <option value="sse">sse</option>
          </Select>
        </label>
      </div>

      {transport === 'stdio' ? (
        <div className="grid grid-cols-2 gap-3">
          <label className="block">
            <span className="mb-1 block text-[12.5px] text-ink-soft">Command</span>
            <input
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              placeholder="/usr/bin/my-mcp-server"
              className="h-8 w-full rounded border border-line bg-surface px-2 font-mono text-[12.5px] text-ink placeholder:text-ink-faint"
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-[12.5px] text-ink-soft">Arguments</span>
            <input
              value={args}
              onChange={(e) => setArgs(e.target.value)}
              placeholder="--root /home/me/notes"
              className="h-8 w-full rounded border border-line bg-surface px-2 font-mono text-[12.5px] text-ink placeholder:text-ink-faint"
            />
          </label>
        </div>
      ) : (
        <label className="block">
          <span className="mb-1 block text-[12.5px] text-ink-soft">URL</span>
          <input
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://example.com/mcp"
            className="h-8 w-full rounded border border-line bg-surface px-2 font-mono text-[12.5px] text-ink placeholder:text-ink-faint"
          />
        </label>
      )}

      <label className="block">
        <span className="mb-1 block text-[12.5px] text-ink-soft">Available in</span>
        <Select
          value={projectId}
          aria-label="Which conversations may use this server"
          onChange={(e) => setProjectId(e.target.value)}
        >
          <option value="">All conversations</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              Only the “{p.name}” project
            </option>
          ))}
        </Select>
        <span className="mt-1 block text-[12px] text-ink-faint">
          Enabling a server grants its approved tools to every conversation that can see it.
          Scoping it to a project keeps it out of the rest.
        </span>
      </label>

      <div className="flex justify-end gap-2">
        <Button size="sm" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button size="sm" disabled={name.trim() === ''} onClick={() => void submit()}>
          Add server
        </Button>
      </div>
    </div>
  );
}
