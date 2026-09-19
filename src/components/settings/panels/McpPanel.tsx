import { Plug } from 'lucide-react';
import { openExternal } from '@/lib/external';

/**
 * MCP is Phase 3. The database schema (`mcp_servers`, `mcp_permissions`) and
 * this section already exist so the shape of the feature is fixed, but no
 * server is launched yet — showing a configuration UI that silently does
 * nothing would be worse than saying so.
 */
export function McpPanel() {
  return (
    <div className="rounded-lg border border-line bg-sunken/40 p-5">
      <div className="mb-3 flex items-center gap-2.5">
        <span className="flex size-9 items-center justify-center rounded-lg bg-accent-soft">
          <Plug size={17} className="text-accent" aria-hidden />
        </span>
        <div>
          <h4 className="text-[14px] font-semibold text-ink">Model Context Protocol</h4>
          <p className="text-[12.5px] text-ink-faint">Planned for the next milestone.</p>
        </div>
      </div>

      <p className="mb-3 max-w-prose text-[13px] leading-relaxed text-ink-soft">
        MCP will let Claude use tools you explicitly grant — reading a project folder, querying a
        database, working with a repository. The groundwork is in place: server definitions and
        per-tool permissions already have their tables in the local database, and every tool call
        will require approval before it runs.
      </p>

      <ul className="mb-4 space-y-1.5 text-[13px] text-ink-soft">
        <li className="flex gap-2"><span className="text-ink-faint">•</span> Configure stdio, SSE and HTTP servers</li>
        <li className="flex gap-2"><span className="text-ink-faint">•</span> Per-project server sets and defaults</li>
        <li className="flex gap-2"><span className="text-ink-faint">•</span> Allow once / always for this project / deny, per tool</li>
        <li className="flex gap-2"><span className="text-ink-faint">•</span> Tool calls and results rendered inline, collapsible</li>
      </ul>

      <button
        type="button"
        onClick={() => void openExternal('https://modelcontextprotocol.io')}
        className="text-[13px] text-accent hover:underline"
      >
        Learn about MCP →
      </button>
    </div>
  );
}
