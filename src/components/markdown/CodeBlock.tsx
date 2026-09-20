import { memo, useEffect, useRef, useState } from 'react';
import { Check, Copy, Download, PanelRight, WrapText } from 'lucide-react';
import { highlight, languageLabel, MAX_HIGHLIGHT_CHARS } from '@/lib/highlighter';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { cn } from '@/lib/cn';
import { IconButton } from '@/components/ui/IconButton';
import { writeTextFile } from '@/services/api';

interface Props {
  code: string;
  language?: string;
  /** True while the enclosing message is still streaming. */
  live?: boolean;
  /** Rendered inside the artifact panel, which must not offer to reopen it. */
  inPanel?: boolean;
}

/** Enough lines that reading it in the transcript is a chore. */
const PANEL_WORTHY_LINES = 12;

/**
 * A fenced code block: language label, copy, wrap toggle and save.
 *
 * Highlighting is asynchronous and always has a plain-text fallback already
 * on screen, so a slow grammar load never leaves a blank box — and while a
 * reply streams we deliberately skip highlighting entirely, because
 * re-tokenising a growing, syntactically incomplete block every frame is both
 * expensive and visually noisy.
 */
export const CodeBlock = memo(function CodeBlock({ code, language, live, inPanel }: Props) {
  const [html, setHtml] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const globalWrap = useSettingsStore((s) => s.settings?.['appearance.codeWrap'] ?? false);
  const [wrap, setWrap] = useState(globalWrap);
  const toast = useUIStore((s) => s.toast);
  const openArtifact = useUIStore((s) => s.openArtifact);
  const copyTimer = useRef<number | null>(null);

  useEffect(() => setWrap(globalWrap), [globalWrap]);

  useEffect(() => {
    if (live || code.length > MAX_HIGHLIGHT_CHARS) {
      setHtml(null);
      return;
    }
    let cancelled = false;
    void highlight(code, language).then((out) => {
      if (!cancelled) setHtml(out);
    });
    return () => {
      cancelled = true;
    };
  }, [code, language, live]);

  useEffect(
    () => () => {
      if (copyTimer.current) window.clearTimeout(copyTimer.current);
    },
    [],
  );

  const label = languageLabel(language);
  const lineCount = code.split('\n').length;

  async function copy() {
    try {
      const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
      await writeText(code);
      setCopied(true);
      copyTimer.current = window.setTimeout(() => setCopied(false), 1600);
    } catch {
      toast('error', 'Could not copy to the clipboard.');
    }
  }

  async function save() {
    try {
      const { save: saveDialog } = await import('@tauri-apps/plugin-dialog');
      const ext = extensionFor(language);
      const path = await saveDialog({ defaultPath: `snippet${ext}` });
      if (!path) return;
      await writeTextFile(path, code);
      toast('success', 'Snippet saved.');
    } catch {
      toast('error', 'Could not save the snippet.');
    }
  }

  return (
    <figure className="group/code my-3 overflow-hidden rounded-lg border border-line bg-sunken">
      <figcaption className="flex items-center justify-between gap-2 border-b border-line bg-surface/60 px-3 py-1.5">
        <span className="select-none font-mono text-[11px] uppercase tracking-wide text-ink-faint">
          {label}
          {lineCount > 1 && (
            <span className="ml-2 normal-case tracking-normal">{lineCount} lines</span>
          )}
        </span>
        <div className="flex items-center gap-0.5 opacity-60 transition-opacity focus-within:opacity-100 group-hover/code:opacity-100">
          <IconButton
            label={wrap ? 'Disable word wrap' : 'Enable word wrap'}
            onClick={() => setWrap((w) => !w)}
            active={wrap}
            size="sm"
          >
            <WrapText size={14} />
          </IconButton>
          {!inPanel && !live && lineCount >= PANEL_WORTHY_LINES && (
            <IconButton
              label="Open in side panel"
              size="sm"
              onClick={() =>
                openArtifact({
                  title: `snippet.${language && /^[a-z0-9]+$/i.test(language) ? language : 'txt'}`,
                  code,
                  language,
                })
              }
            >
              <PanelRight size={14} />
            </IconButton>
          )}
          <IconButton label="Save snippet to a file" onClick={save} size="sm">
            <Download size={14} />
          </IconButton>
          <IconButton label={copied ? 'Copied' : 'Copy code'} onClick={copy} size="sm">
            {copied ? <Check size={14} className="text-success" /> : <Copy size={14} />}
          </IconButton>
        </div>
      </figcaption>

      <div className={cn('overflow-x-auto scroll-thin', wrap && 'code-wrap')}>
        {html ? (
          // Shiki output only ever contains <pre>/<code>/<span> with styles;
          // the source text is escaped by the tokenizer.
          <div className="[&_pre]:!m-0 [&_pre]:!bg-transparent [&_pre]:p-3" dangerouslySetInnerHTML={{ __html: html }} />
        ) : (
          <pre className="m-0 p-3">
            <code
              className={cn(
                'font-mono text-[0.86em] leading-relaxed',
                wrap && 'whitespace-pre-wrap break-words',
              )}
            >
              {code}
            </code>
          </pre>
        )}
      </div>
    </figure>
  );
});

function extensionFor(language?: string): string {
  const map: Record<string, string> = {
    javascript: '.js', js: '.js', typescript: '.ts', ts: '.ts', tsx: '.tsx', jsx: '.jsx',
    python: '.py', py: '.py', rust: '.rs', rs: '.rs', go: '.go', java: '.java',
    ruby: '.rb', rb: '.rb', php: '.php', c: '.c', cpp: '.cpp', csharp: '.cs',
    shell: '.sh', bash: '.sh', sh: '.sh', sql: '.sql', json: '.json',
    yaml: '.yaml', yml: '.yaml', toml: '.toml', html: '.html', css: '.css',
    markdown: '.md', md: '.md', xml: '.xml', swift: '.swift', kotlin: '.kt',
  };
  return map[(language ?? '').toLowerCase()] ?? '.txt';
}
