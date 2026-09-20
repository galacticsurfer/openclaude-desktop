import { describe, expect, it } from 'vitest';
import { matchCommands, slashQuery } from './SlashCommandMenu';

describe('slashQuery', () => {
  it('opens only on a leading slash', () => {
    expect(slashQuery('/')).toBe('');
    expect(slashQuery('/mod')).toBe('mod');
    // A slash inside a sentence is just text, not a command.
    expect(slashQuery('what is and/or')).toBeNull();
    expect(slashQuery('')).toBeNull();
    expect(slashQuery('hello')).toBeNull();
  });

  it('stops suggesting once the command has an argument', () => {
    expect(slashQuery('/model ')).toBeNull();
    expect(slashQuery('/model opus')).toBeNull();
  });
});

describe('matchCommands', () => {
  const all = ['compact', 'context', 'model', 'code-review', 'config', 'rename'];

  it('prefers prefix matches over substring matches', () => {
    // "re" prefixes only "rename"; "code-review" merely contains it, so it
    // ranks below even though it comes first in the source list.
    expect(matchCommands(all, 're')).toEqual(['rename', 'code-review']);
  });

  it('keeps source order within the prefix group', () => {
    // All four begin with "co"; none should be reordered among themselves.
    expect(matchCommands(all, 'co')).toEqual([
      'compact',
      'context',
      'code-review',
      'config',
    ]);
  });

  it('is case-insensitive', () => {
    expect(matchCommands(all, 'MOD')).toEqual(['model']);
  });

  it('returns everything for a bare slash', () => {
    expect(matchCommands(all, '')).toHaveLength(all.length);
  });

  it('returns nothing when there is no match', () => {
    expect(matchCommands(all, 'zzz')).toEqual([]);
  });
});
