import { memo, useEffect, useRef, useState } from 'react';
import { AlertTriangle, Code2, Eye, PanelRight } from 'lucide-react';
import { CodeBlock } from './CodeBlock';
import { useUIStore } from '@/stores/useUIStore';

/**
 * A rendered Mermaid diagram, with the source one click away.
 *
 * Mermaid is ~2MB, so it is imported on demand: a conversation without a
 * diagram never pays for it. Rendering is deliberately never attempted on a
 * streaming block — half-written diagram source is a syntax error by
 * definition, and flashing a parse failure on every frame is worse than
 * showing the text until it settles.
 */

let loader: Promise<typeof import('mermaid')['default']> | null = null;

/** Load and configure Mermaid once, on first use. */
function mermaid(dark: boolean) {
  loader ??= import('mermaid').then((m) => m.default);
  return loader.then((m) => {
    m.initialize({
      startOnLoad: false,
      theme: dark ? 'dark' : 'default',
      // Diagram source comes from the model: never let it inject markup.
      securityLevel: 'strict',
      fontFamily: 'inherit',
    });
    return m;
  });
}

function isDark() {
  return document.documentElement.classList.contains('dark');
}

export const Mermaid = memo(function Mermaid({
  code,
  inPanel,
}: {
  code: string;
  /** Rendered inside the artifact panel, which must not offer to reopen it. */
  inPanel?: boolean;
}) {
  const [svg, setSvg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showSource, setShowSource] = useState(false);
  const [dark, setDark] = useState(isDark);
  const idRef = useRef(`mmd-${Math.random().toString(36).slice(2)}`);
  const openArtifact = useUIStore((s) => s.openArtifact);

  // Mermaid bakes the palette into the SVG, so a theme change needs a
  // re-render rather than a restyle.
  useEffect(() => {
    const observer = new MutationObserver(() => setDark(isDark()));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    let cancelled = false;
    setError(null);

    void mermaid(dark)
      .then((m) => m.render(idRef.current, code))
      .then(({ svg: out }) => {
        if (!cancelled) setSvg(out);
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        setSvg(null);
        setError(e instanceof Error ? e.message : 'This diagram could not be drawn.');
      });

    return () => {
      cancelled = true;
    };
  }, [code, dark]);

  if (error !== null) {
    return (
      <div className="my-3 overflow-hidden rounded-lg border border-line">
        <p className="flex items-start gap-1.5 border-b border-line bg-sunken px-3 py-1.5 text-[12px] text-ink-faint">
          <AlertTriangle size={13} className="mt-px shrink-0 text-warn" aria-hidden />
          <span>Mermaid could not draw this diagram — showing the source.</span>
        </p>
        <CodeBlock code={code} language="mermaid" />
      </div>
    );
  }

  return (
    <div className="my-3 overflow-hidden rounded-lg border border-line">
      <div className="flex items-center gap-2 border-b border-line bg-sunken px-3 py-1">
        <span className="font-mono text-[11px] uppercase tracking-wide text-ink-faint">
          Diagram
        </span>
        {!inPanel && (
          <button
            type="button"
            onClick={() => openArtifact({ title: 'diagram.mmd', code, language: 'mermaid' })}
            className="ml-auto flex items-center gap-1 rounded px-1.5 py-0.5 text-[11.5px] text-ink-faint hover:bg-line hover:text-ink"
          >
            <PanelRight size={12} />
            Panel
          </button>
        )}
        <button
          type="button"
          onClick={() => setShowSource((v) => !v)}
          className={`${inPanel ? 'ml-auto ' : ''}flex items-center gap-1 rounded px-1.5 py-0.5 text-[11.5px] text-ink-faint hover:bg-line hover:text-ink`}
        >
          {showSource ? <Eye size={12} /> : <Code2 size={12} />}
          {showSource ? 'Diagram' : 'Source'}
        </button>
      </div>

      {showSource ? (
        <CodeBlock code={code} language="mermaid" />
      ) : svg === null ? (
        <p className="px-3 py-6 text-center text-[12.5px] text-ink-faint">Drawing diagram…</p>
      ) : (
        // Mermaid renders with securityLevel 'strict', which strips scripts
        // and event handlers from the diagram source before it becomes SVG.
        <div
          className="mermaid-figure overflow-x-auto scroll-thin px-3 py-3"
          dangerouslySetInnerHTML={{ __html: svg }}
        />
      )}
    </div>
  );
});
