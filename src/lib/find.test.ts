import { describe, expect, it } from 'vitest';
import { findInMessages, stepMatch } from './find';
import type { Message } from '@/types';

const msg = (id: string, content: string, thinking: string | null = null) =>
  ({ id, content, thinking }) as Message;

describe('findInMessages', () => {
  const messages = [
    msg('a', 'Rivers begin as small things'),
    msg('b', 'Nothing relevant here'),
    msg('c', 'A RIVER in caps'),
    msg('d', 'no match', 'hidden river in reasoning'),
  ];

  it('finds nothing for an empty or blank query', () => {
    expect(findInMessages(messages, '')).toEqual([]);
    expect(findInMessages(messages, '   ')).toEqual([]);
  });

  it('matches case-insensitively', () => {
    expect(findInMessages(messages, 'river').map((m) => m.messageId)).toEqual(['a', 'c', 'd']);
  });

  it('matches partial words, unlike the stemming FTS index', () => {
    expect(findInMessages(messages, 'iver').map((m) => m.messageId)).toEqual(['a', 'c', 'd']);
  });

  it('searches reasoning text as well as the reply', () => {
    expect(findInMessages(messages, 'hidden').map((m) => m.messageId)).toEqual(['d']);
  });

  it('returns matches in transcript order with their positions', () => {
    expect(findInMessages(messages, 'river')).toEqual([
      { messageId: 'a', index: 0 },
      { messageId: 'c', index: 2 },
      { messageId: 'd', index: 3 },
    ]);
  });
});

describe('stepMatch', () => {
  it('wraps forward past the end', () => {
    expect(stepMatch(2, 3, 1)).toBe(0);
  });
  it('wraps backward past the start', () => {
    expect(stepMatch(0, 3, -1)).toBe(2);
  });
  it('stays put when there is nothing to step through', () => {
    expect(stepMatch(0, 0, 1)).toBe(0);
  });
});
