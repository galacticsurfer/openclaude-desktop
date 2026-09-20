import { useState } from 'react';
import { Check, Copy, Download, X } from 'lucide-react';
import { useUIStore } from '@/stores/useUIStore';
import { IconButton } from '@/components/ui/IconButton';
import { CodeBlock } from '@/components/markdown/CodeBlock';
import { Mermaid } from '@/components/markdown/Mermaid';
import { languageLabel } from '@/lib/highlighter';
import { writeTextFile } from '@/services/api';
import { AppError } from '@/services/ipc';

/**
 * A code block or diagram given room to breathe, beside the conversation.
 *
 * Explicitly not claude.ai's artifacts: nothing here executes. Long output
 * is simply hard to read in a scrolling transcript, and this is the cheap
 * honest fix — the same content, pinned, full height, with save and copy.
 */
export function ArtifactPanel() {
  const artifact = useUIStore((s) => s.artifact);
  const close = useUIStore((s) => s.closeArtifact);
  const toast = useUIStore((s) => s.toast);
  const [copied, setCopied] = useState(false);

  if (!artifact) return null;
  const { title, code, language } = artifact;

  async function copy() {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  }

  async function save() {
    try {
      const { save: pick } = await import('@tauri-apps/plugin-dialog');
      const path = await pick({ defaultPath: title });
      if (typeof path === 'string') {
        await writeTextFile(path, code);
        toast('success', 'Saved.');
      }
    } catch (err) {
      toast('error', AppError.from(err).message);
    }
  }

  return (
    <aside
      aria-label="Artifact"
      className="flex h-full w-[38%] min-w-[22rem] shrink-0 flex-col border-l border-line bg-surface"
    >
      <header className="flex items-center gap-2 border-b border-line px-3 py-2">
        <div className="min-w-0">
          <p className="truncate text-[13px] font-medium text-ink">{title}</p>
          <p className="text-[11.5px] text-ink-faint">
            {language === 'mermaid' ? 'Diagram' : languageLabel(language)}
          </p>
        </div>
        <div className="ml-auto flex items-center gap-0.5">
          <IconButton label={copied ? 'Copied' : 'Copy'} size="sm" onClick={() => void copy()}>
            {copied ? <Check size={14} className="text-success" /> : <Copy size={14} />}
          </IconButton>
          <IconButton label="Save to a file" size="sm" onClick={() => void save()}>
            <Download size={14} />
          </IconButton>
          <IconButton label="Close panel" size="sm" onClick={close}>
            <X size={14} />
          </IconButton>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto scroll-thin p-3">
        {language === 'mermaid' ? (
          <Mermaid code={code} inPanel />
        ) : (
          <CodeBlock code={code} language={language} inPanel />
        )}
      </div>
    </aside>
  );
}
