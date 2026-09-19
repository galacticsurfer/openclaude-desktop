/**
 * Lazy Shiki highlighter.
 *
 * Shiki's full bundle is tens of megabytes of grammars, which would dominate
 * startup. Instead the core is imported on first use, the JavaScript regex
 * engine is used (no WASM download, faster cold start), and each language
 * grammar is fetched only when a code block actually needs it.
 */

import type { HighlighterCore } from 'shiki/core';

/** Themes are loaded as a pair so the two app themes share one render. */
const LIGHT_THEME = 'github-light';
const DARK_THEME = 'github-dark';

let highlighterPromise: Promise<HighlighterCore> | null = null;
const loaded = new Set<string>();
const loading = new Map<string, Promise<void>>();

/**
 * Aliases and the grammar that serves them. Anything not listed renders as
 * plain text rather than failing.
 */
const ALIASES: Record<string, string> = {
  js: 'javascript', jsx: 'jsx', mjs: 'javascript', cjs: 'javascript', node: 'javascript',
  ts: 'typescript', tsx: 'tsx',
  py: 'python', python3: 'python',
  rb: 'ruby', rs: 'rust', golang: 'go',
  sh: 'shell', bash: 'shell', zsh: 'shell', shell: 'shell', console: 'shell', terminal: 'shell',
  yml: 'yaml', md: 'markdown', mdx: 'markdown',
  'c++': 'cpp', cc: 'cpp', cxx: 'cpp', hpp: 'cpp', h: 'c',
  'c#': 'csharp', cs: 'csharp',
  dockerfile: 'docker', docker: 'docker',
  postgres: 'sql', postgresql: 'sql', mysql: 'sql', sqlite: 'sql',
  html: 'html', htm: 'html', xml: 'xml', svg: 'xml',
  kt: 'kotlin', tf: 'terraform', hcl: 'hcl',
  txt: 'text', text: 'text', plain: 'text', '': 'text',
};

/** Grammar modules, lazily imported. Keys are canonical language ids. */
const GRAMMARS: Record<string, () => Promise<unknown>> = {
  javascript: () => import('shiki/langs/javascript.mjs'),
  jsx: () => import('shiki/langs/jsx.mjs'),
  typescript: () => import('shiki/langs/typescript.mjs'),
  tsx: () => import('shiki/langs/tsx.mjs'),
  python: () => import('shiki/langs/python.mjs'),
  rust: () => import('shiki/langs/rust.mjs'),
  go: () => import('shiki/langs/go.mjs'),
  java: () => import('shiki/langs/java.mjs'),
  kotlin: () => import('shiki/langs/kotlin.mjs'),
  ruby: () => import('shiki/langs/ruby.mjs'),
  php: () => import('shiki/langs/php.mjs'),
  c: () => import('shiki/langs/c.mjs'),
  cpp: () => import('shiki/langs/cpp.mjs'),
  csharp: () => import('shiki/langs/csharp.mjs'),
  swift: () => import('shiki/langs/swift.mjs'),
  shell: () => import('shiki/langs/shellscript.mjs'),
  sql: () => import('shiki/langs/sql.mjs'),
  json: () => import('shiki/langs/json.mjs'),
  yaml: () => import('shiki/langs/yaml.mjs'),
  toml: () => import('shiki/langs/toml.mjs'),
  xml: () => import('shiki/langs/xml.mjs'),
  html: () => import('shiki/langs/html.mjs'),
  css: () => import('shiki/langs/css.mjs'),
  scss: () => import('shiki/langs/scss.mjs'),
  markdown: () => import('shiki/langs/markdown.mjs'),
  diff: () => import('shiki/langs/diff.mjs'),
  docker: () => import('shiki/langs/docker.mjs'),
  lua: () => import('shiki/langs/lua.mjs'),
  r: () => import('shiki/langs/r.mjs'),
  scala: () => import('shiki/langs/scala.mjs'),
  elixir: () => import('shiki/langs/elixir.mjs'),
  haskell: () => import('shiki/langs/haskell.mjs'),
  zig: () => import('shiki/langs/zig.mjs'),
  dart: () => import('shiki/langs/dart.mjs'),
  vue: () => import('shiki/langs/vue.mjs'),
  svelte: () => import('shiki/langs/svelte.mjs'),
  graphql: () => import('shiki/langs/graphql.mjs'),
  terraform: () => import('shiki/langs/terraform.mjs'),
  hcl: () => import('shiki/langs/hcl.mjs'),
  ini: () => import('shiki/langs/ini.mjs'),
  perl: () => import('shiki/langs/perl.mjs'),
  powershell: () => import('shiki/langs/powershell.mjs'),
  makefile: () => import('shiki/langs/make.mjs'),
  nix: () => import('shiki/langs/nix.mjs'),
  protobuf: () => import('shiki/langs/proto.mjs'),
};

/** Canonical language id for a fence info string, or null if unsupported. */
export function resolveLanguage(raw: string | undefined): string | null {
  const key = (raw ?? '').trim().toLowerCase().split(/[\s:,{]/)[0] ?? '';
  const canonical = ALIASES[key] ?? key;
  if (canonical === 'text' || canonical === '') return null;
  return canonical in GRAMMARS ? canonical : null;
}

/** Human label for the code block header, even for unhighlighted languages. */
export function languageLabel(raw: string | undefined): string {
  const key = (raw ?? '').trim();
  if (!key) return 'text';
  const pretty: Record<string, string> = {
    javascript: 'JavaScript', typescript: 'TypeScript', jsx: 'JSX', tsx: 'TSX',
    python: 'Python', rust: 'Rust', go: 'Go', java: 'Java', kotlin: 'Kotlin',
    ruby: 'Ruby', php: 'PHP', c: 'C', cpp: 'C++', csharp: 'C#', swift: 'Swift',
    shell: 'Shell', bash: 'Bash', sql: 'SQL', json: 'JSON', yaml: 'YAML',
    toml: 'TOML', xml: 'XML', html: 'HTML', css: 'CSS', scss: 'SCSS',
    markdown: 'Markdown', diff: 'Diff', docker: 'Dockerfile', graphql: 'GraphQL',
    terraform: 'Terraform', haskell: 'Haskell', elixir: 'Elixir', scala: 'Scala',
  };
  const canonical = ALIASES[key.toLowerCase()] ?? key.toLowerCase();
  return pretty[canonical] ?? key;
}

async function getHighlighter(): Promise<HighlighterCore> {
  if (!highlighterPromise) {
    highlighterPromise = (async () => {
      const [{ createHighlighterCore }, { createJavaScriptRegexEngine }, light, dark] =
        await Promise.all([
          import('shiki/core'),
          import('shiki/engine/javascript'),
          import('shiki/themes/github-light.mjs'),
          import('shiki/themes/github-dark.mjs'),
        ]);
      return createHighlighterCore({
        themes: [light, dark],
        langs: [],
        // `forgiving` keeps an unusual grammar construct from throwing and
        // blanking a code block mid-stream.
        engine: createJavaScriptRegexEngine({ forgiving: true }),
      });
    })();
  }
  return highlighterPromise;
}

async function ensureLanguage(lang: string): Promise<boolean> {
  if (loaded.has(lang)) return true;
  const load = GRAMMARS[lang];
  if (!load) return false;

  let inflight = loading.get(lang);
  if (!inflight) {
    inflight = (async () => {
      const [hl, mod] = await Promise.all([getHighlighter(), load()]);
      await hl.loadLanguage((mod as { default: never }).default);
      loaded.add(lang);
    })();
    loading.set(lang, inflight);
  }

  try {
    await inflight;
    return true;
  } catch (err) {
    console.warn(`[shiki] could not load grammar "${lang}"`, err);
    return false;
  } finally {
    loading.delete(lang);
  }
}

/**
 * Highlight to HTML, or return null when the language is unsupported or the
 * highlighter fails — callers then render plain text, which is always correct.
 */
export async function highlight(code: string, rawLang: string | undefined): Promise<string | null> {
  const lang = resolveLanguage(rawLang);
  if (!lang) return null;
  if (!(await ensureLanguage(lang))) return null;

  try {
    const hl = await getHighlighter();
    return hl.codeToHtml(code, {
      lang,
      themes: { light: LIGHT_THEME, dark: DARK_THEME },
      // Emits `--shiki-dark` custom properties that index.css swaps in, so a
      // theme change needs no re-highlight.
      defaultColor: false,
      colorReplacements: {},
    });
  } catch (err) {
    console.warn('[shiki] highlight failed', err);
    return null;
  }
}

/** Very large blocks are left unhighlighted; tokenising them stalls the UI. */
export const MAX_HIGHLIGHT_CHARS = 100_000;
