import { describe, expect, it } from 'vitest';
import { splitSettled } from './StreamingMarkdown';

describe('splitSettled', () => {
  it('keeps a single unfinished paragraph entirely in the tail', () => {
    const { settled, tail } = splitSettled('Here is a partial sen');
    expect(settled).toBe('');
    expect(tail).toBe('Here is a partial sen');
  });

  it('settles a paragraph once a blank line follows it', () => {
    const { settled, tail } = splitSettled('First para.\n\nSecond par');
    expect(settled).toBe('First para.\n');
    expect(tail).toBe('Second par');
  });

  it('never splits inside a fence, where blank lines are content', () => {
    // Splitting here would leave an unterminated fence in the settled half
    // and render the code as prose.
    const text = '```python\ndef f():\n\n    return 1\n';
    const { settled, tail } = splitSettled(text);
    expect(settled).toBe('');
    expect(tail).toBe(text);
  });

  it('settles a fence as soon as it closes', () => {
    const text = '```python\nprint(1)\n```\nAfter the bl';
    const { settled, tail } = splitSettled(text);
    expect(settled).toBe('```python\nprint(1)\n```');
    expect(tail).toBe('After the bl');
  });

  it('advances the boundary as more blocks complete', () => {
    const a = splitSettled('One.\n\nTwo.\n\nThr');
    expect(a.settled).toBe('One.\n\nTwo.\n');
    expect(a.tail).toBe('Thr');
  });

  it('loses nothing: the two halves always rejoin to the original', () => {
    const samples = [
      '',
      'plain',
      'a\n\nb',
      '```\ncode\n```\ntail',
      '# Heading\n\n- one\n- two\n\ntrailing',
      '```js\nconst a = 1;\n\nconst b = 2;\n',
    ];
    for (const s of samples) {
      const { settled, tail } = splitSettled(s);
      const rejoined = settled === '' ? tail : `${settled}\n${tail}`;
      expect(rejoined).toBe(s);
    }
  });
});
