import { useCallback, useEffect, useRef, useState } from 'react';

/** How close to the bottom still counts as "at the bottom", in pixels. */
const SLACK = 80;
/** How long after a gesture a scroll still counts as user-driven. */
const INTENT_MS = 300;

/**
 * Keep a scroll container pinned to the bottom while content streams in,
 * but stop the moment the user scrolls up to read something.
 *
 * Three things make this harder than it looks:
 *
 *  * The element arrives late. The chat view renders an empty state before a
 *    conversation is open, so a `useRef` bound in a `[]` effect would latch
 *    onto `null` and never attach. Hence the callback ref.
 *
 *  * Growing content fires no `scroll` event, so following a stream cannot be
 *    driven by scroll alone — a `ResizeObserver` does the actual pinning.
 *
 *  * Our own pinning *does* fire `scroll`, asynchronously. Deciding "the user
 *    scrolled away" from those events is wrong: by the time one is delivered
 *    more tokens have grown the container, it reads as a large distance from
 *    the bottom, and the follow is abandoned mid-reply. So un-pinning is
 *    driven only by real input — wheel, touch, scrollbar drag, keys — while
 *    `scroll` may only ever re-pin.
 */
export function useAutoScroll<T extends HTMLElement>(deps: unknown[]) {
  const [node, setNode] = useState<T | null>(null);
  const ref = useCallback((n: T | null) => setNode(n), []);

  const [atBottom, setAtBottom] = useState(true);
  // Mirrors `atBottom`, but readable synchronously from observers.
  const stuck = useRef(true);

  const scrollToBottom = useCallback(
    (behavior: ScrollBehavior = 'smooth') => {
      if (!node) return;
      node.scrollTo({ top: node.scrollHeight, behavior });
      stuck.current = true;
      setAtBottom(true);
    },
    [node],
  );

  useEffect(() => {
    if (!node) return;

    const nearBottom = () =>
      node.scrollHeight - node.scrollTop - node.clientHeight < SLACK;

    let gestureUntil = 0;
    const markGesture = () => {
      gestureUntil = performance.now() + INTENT_MS;
    };

    const onScroll = () => {
      const near = nearBottom();
      setAtBottom(near);
      if (near) {
        // Returning to the bottom always resumes following.
        stuck.current = true;
      } else if (performance.now() < gestureUntil) {
        // Only a user-driven scroll stops it.
        stuck.current = false;
      }
    };

    node.addEventListener('scroll', onScroll, { passive: true });
    node.addEventListener('wheel', markGesture, { passive: true });
    node.addEventListener('touchmove', markGesture, { passive: true });
    node.addEventListener('pointerdown', markGesture, { passive: true });
    node.addEventListener('keydown', markGesture);

    const observer = new ResizeObserver(() => {
      if (stuck.current) node.scrollTop = node.scrollHeight;
      else setAtBottom(nearBottom());
    });
    observer.observe(node);
    // The inner wrapper is what actually grows as messages arrive.
    for (const child of Array.from(node.children)) observer.observe(child);

    setAtBottom(nearBottom());

    return () => {
      node.removeEventListener('scroll', onScroll);
      node.removeEventListener('wheel', markGesture);
      node.removeEventListener('touchmove', markGesture);
      node.removeEventListener('pointerdown', markGesture);
      node.removeEventListener('keydown', markGesture);
      observer.disconnect();
    };
  }, [node]);

  useEffect(() => {
    if (node && stuck.current) node.scrollTop = node.scrollHeight;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [node, ...deps]);

  return { ref, atBottom, scrollToBottom };
}
