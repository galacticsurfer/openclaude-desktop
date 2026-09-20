import type { Message } from '@/types';

export interface FindMatch {
  messageId: string;
  /** Index of the message in the transcript, for ordering. */
  index: number;
}

/**
 * Literal, case-insensitive find over a transcript.
 *
 * Deliberately not the FTS5 index behind Ctrl+K: that tokenises and stems, so
 * searching "run" there also hits "running". Ctrl+F is expected to find
 * exactly what was typed, including partial words and punctuation.
 */
export function findInMessages(messages: Message[], query: string): FindMatch[] {
  const needle = query.trim().toLowerCase();
  if (needle === '') return [];

  const out: FindMatch[] = [];
  messages.forEach((m, index) => {
    const hay = `${m.content}\n${m.thinking ?? ''}`.toLowerCase();
    if (hay.includes(needle)) out.push({ messageId: m.id, index });
  });
  return out;
}

/** Step through matches, wrapping at both ends. */
export function stepMatch(current: number, total: number, delta: number): number {
  if (total <= 0) return 0;
  return (((current + delta) % total) + total) % total;
}
