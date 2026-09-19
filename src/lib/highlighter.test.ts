import { describe, expect, it } from 'vitest';
import { languageLabel, resolveLanguage } from './highlighter';

describe('resolveLanguage', () => {
  it('maps common aliases to a bundled grammar', () => {
    expect(resolveLanguage('js')).toBe('javascript');
    expect(resolveLanguage('py')).toBe('python');
    expect(resolveLanguage('rs')).toBe('rust');
    expect(resolveLanguage('bash')).toBe('shell');
    expect(resolveLanguage('yml')).toBe('yaml');
    expect(resolveLanguage('c++')).toBe('cpp');
  });

  it('ignores case and trailing fence metadata', () => {
    expect(resolveLanguage('Python')).toBe('python');
    expect(resolveLanguage('ts {1,3}')).toBe('typescript');
    expect(resolveLanguage('js:title=app.js')).toBe('javascript');
  });

  it('returns null for plain text and unknown languages', () => {
    // A null result means "render as plain text", never an error.
    expect(resolveLanguage(undefined)).toBeNull();
    expect(resolveLanguage('')).toBeNull();
    expect(resolveLanguage('text')).toBeNull();
    expect(resolveLanguage('brainfuck')).toBeNull();
  });
});

describe('languageLabel', () => {
  it('shows a readable name in the code block header', () => {
    expect(languageLabel('ts')).toBe('TypeScript');
    expect(languageLabel('cpp')).toBe('C++');
    expect(languageLabel(undefined)).toBe('text');
    // Unknown languages still get labelled with whatever the author wrote.
    expect(languageLabel('cobol')).toBe('cobol');
  });
});
