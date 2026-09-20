import { useEffect, useRef, useState } from 'react';

/**
 * Reveal streamed text at frame rate instead of at arrival rate.
 *
 * The Claude Code CLI does not emit a token at a time: measured over a long
 * reply it sends a ~25-character lump every ~60ms (p50 59ms, p90 61ms). That
 * is roughly sixteen visual updates a second, each one a whole word or two
 * appearing at once — which reads as stuttering no matter how fast the
 * renderer is. Optimising the render path cannot fix it, because the
 * chunkiness is in the arrival pattern, not the paint.
 *
 * So arrival and display are decoupled: deltas accumulate in the store, and
 * this hook walks a cursor through them once per animation frame. The cursor
 * speed is not fixed — it drains whatever is waiting over CATCH_UP_MS, so it
 * automatically matches however fast Claude happens to be generating and the
 * text never falls more than about a frame's worth behind.
 */

/** How long the reveal takes to drain the text already received. */
export const CATCH_UP_MS = 110;

/** Below this a slow trickle would visibly crawl rather than flow. */
const MIN_CHARS_PER_FRAME = 1;

/** Characters to uncover this frame, given how many are waiting. */
export function revealStep(pending: number, dtMs: number, catchUpMs = CATCH_UP_MS): number {
  if (pending <= 0) return 0;
  const share = (pending * dtMs) / catchUpMs;
  return Math.min(pending, Math.max(MIN_CHARS_PER_FRAME, Math.ceil(share)));
}

/**
 * `target` revealed progressively while `enabled`, or in full when not.
 *
 * Returns the whole string the moment streaming stops, so a finished message
 * never sits half-drawn.
 */
export function useSmoothText(target: string, enabled: boolean): string {
  const [count, setCount] = useState(target.length);
  const targetRef = useRef(target);
  const countRef = useRef(count);
  targetRef.current = target;
  countRef.current = count;

  useEffect(() => {
    if (!enabled) return;
    let frame = 0;
    let last = performance.now();

    const tick = (now: number) => {
      // A backgrounded tab can hand us a gap of seconds; treat that as one
      // frame rather than dumping the whole buffer in a single jump.
      const dt = Math.min(now - last, 100);
      last = now;
      const step = revealStep(targetRef.current.length - countRef.current, dt);
      if (step > 0) {
        countRef.current += step;
        setCount(countRef.current);
      }
      frame = requestAnimationFrame(tick);
    };

    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [enabled]);

  if (!enabled) return target;
  // Retrying or starting a new reply shrinks the target under the cursor.
  if (count > target.length) return target;
  return target.slice(0, count);
}
