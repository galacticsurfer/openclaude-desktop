import { memo, useMemo } from 'react';
import { Markdown } from './Markdown';

/**
 * Markdown for a reply that is still arriving.
 *
 * Rendering the whole message through the parser on every frame is O(n) per
 * frame and therefore O(n²) over a reply — which is exactly why a long
 * answer starts smooth and gets choppier the longer it runs.
 *
 * Markdown is block-structured, so most of a partial document is already
 * final: only the block currently being written can still change. This
 * splits the text at the last point that cannot be altered by what comes
 * next, parses the settled part (memoised, so it re-parses only when a block
 * completes), and renders the growing tail as plain text. The expensive work
 * then happens once per completed block instead of once per frame.
 */

/** A boundary is a blank line outside a fence, or a line that closes one. */
export function splitSettled(text: string): { settled: string; tail: string } {
  const lines = text.split('\n');
  let inFence = false;
  // Index of the first line belonging to the unsettled tail.
  let boundary = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i] ?? '';
    const fence = /^\s{0,3}(```|~~~)/.test(line);

    if (fence) {
      if (inFence) {
        // The fence just closed, so everything through it is final.
        inFence = false;
        boundary = i + 1;
      } else {
        inFence = true;
      }
      continue;
    }

    // A blank line ends a block — but only outside a fence, where blank
    // lines are ordinary content.
    if (!inFence && line.trim() === '') boundary = i + 1;
  }

  if (boundary <= 0) return { settled: '', tail: text };
  return {
    settled: lines.slice(0, boundary).join('\n'),
    tail: lines.slice(boundary).join('\n'),
  };
}

export const StreamingMarkdown = memo(function StreamingMarkdown({
  children,
  caret = false,
}: {
  children: string;
  /** Draw the blinking cursor after the last character. */
  caret?: boolean;
}) {
  const { settled, tail } = useMemo(() => splitSettled(children), [children]);

  return (
    <>
      {/* Memoised on `settled`, which changes only when a block completes. */}
      {settled !== '' && <Markdown live>{settled}</Markdown>}
      {(tail !== '' || caret) && (
        // Plain text: cheap, and it only lasts until the block closes. The
        // caret lives in here rather than after the block, or it would wrap
        // onto a line of its own instead of trailing the last character.
        <div
          className={
            settled === ''
              ? 'prose-oc whitespace-pre-wrap'
              : 'prose-oc stream-tail whitespace-pre-wrap'
          }
        >
          {tail}
          {caret && <span className="stream-caret animate-caret" aria-hidden />}
        </div>
      )}
    </>
  );
});
