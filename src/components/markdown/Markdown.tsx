import { memo, type ReactNode } from 'react';
import ReactMarkdown, { type Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { CodeBlock } from './CodeBlock';
import { Mermaid } from './Mermaid';
import { openExternal } from '@/lib/external';

interface Props {
  children: string;
  /** True while this message is still streaming. */
  live?: boolean;
}

function textOf(node: ReactNode): string {
  if (node == null || node === false) return '';
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(textOf).join('');
  if (typeof node === 'object' && 'props' in (node as never)) {
    return textOf((node as { props: { children?: ReactNode } }).props.children);
  }
  return '';
}

/**
 * Markdown renderer.
 *
 * Raw HTML is deliberately *not* enabled: `rehype-raw` is absent, so model
 * output can never inject markup into the app. Links are intercepted and
 * handed to the system browser instead of navigating the webview.
 */
export const Markdown = memo(function Markdown({ children, live }: Props) {
  const components: Components = {
    code({ className, children: kids, ...props }) {
      const match = /language-(\S+)/.exec(className ?? '');
      const raw = textOf(kids);

      // react-markdown gives inline code no language class and no newline.
      const isBlock = Boolean(match) || raw.includes('\n');
      if (!isBlock) {
        return (
          <code className={className} {...props}>
            {kids}
          </code>
        );
      }
      return (
        match?.[1] === 'mermaid' && !live ? (
          <Mermaid code={raw.replace(/\n$/, '')} />
        ) : (
          <CodeBlock code={raw.replace(/\n$/, '')} language={match?.[1]} live={live} />
        )
      );
    },

    // `pre` is handled entirely by CodeBlock; unwrap it so we don't nest.
    pre({ children: kids }) {
      return <>{kids}</>;
    },

    a({ href, children: kids, ...props }) {
      return (
        <a
          href={href}
          onClick={(e) => {
            e.preventDefault();
            if (href) void openExternal(href);
          }}
          title={href}
          {...props}
        >
          {kids}
        </a>
      );
    },

    // Wide tables scroll inside their own container so the page never does.
    table({ children: kids }) {
      return (
        <div className="table-wrap scroll-thin">
          <table>{kids}</table>
        </div>
      );
    },

    input({ type, checked, ...props }) {
      // GFM task list items; read-only, they mirror the model's text.
      if (type === 'checkbox') {
        return (
          <input
            type="checkbox"
            checked={checked}
            readOnly
            className="mr-1.5 translate-y-[1px] accent-[rgb(var(--c-accent))]"
            {...props}
          />
        );
      }
      return <input type={type} {...props} />;
    },
  };

  return (
    <div className="prose-oc">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {children}
      </ReactMarkdown>
    </div>
  );
});
