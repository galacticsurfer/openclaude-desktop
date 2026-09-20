import { beforeEach, describe, expect, it } from 'vitest';
import { useUIStore } from './useUIStore';

const tabs = () => useUIStore.getState().tabs;

describe('conversation tabs', () => {
  beforeEach(() => useUIStore.setState({ tabs: [] }));

  it('opens the first conversation into a tab of its own', () => {
    useUIStore.getState().openInTab('a', null);
    expect(tabs()).toEqual(['a']);
  });

  it('replaces what the active tab shows, rather than piling up', () => {
    useUIStore.getState().openInTab('a', null);
    useUIStore.getState().openInTab('b', 'a');
    expect(tabs()).toEqual(['b']);
  });

  it('focuses the existing tab instead of opening a duplicate', () => {
    useUIStore.setState({ tabs: ['a', 'b'] });
    useUIStore.getState().openInTab('a', 'b');
    expect(tabs()).toEqual(['a', 'b']);
  });

  it('opens a new tab beside the active one, not at the end', () => {
    useUIStore.setState({ tabs: ['a', 'b', 'c'] });
    useUIStore.getState().openInNewTab('d', 'a');
    expect(tabs()).toEqual(['a', 'd', 'b', 'c']);
  });

  it('closing an inactive tab leaves the user where they are', () => {
    useUIStore.setState({ tabs: ['a', 'b', 'c'] });
    expect(useUIStore.getState().closeTab('c', 'a')).toBe('a');
    expect(tabs()).toEqual(['a', 'b']);
  });

  it('closing the active tab falls to its right-hand neighbour', () => {
    useUIStore.setState({ tabs: ['a', 'b', 'c'] });
    expect(useUIStore.getState().closeTab('b', 'b')).toBe('c');
  });

  it('closing the last tab falls left instead', () => {
    useUIStore.setState({ tabs: ['a', 'b'] });
    expect(useUIStore.getState().closeTab('b', 'b')).toBe('a');
  });

  it('closing the only tab leaves nothing open', () => {
    useUIStore.setState({ tabs: ['a'] });
    expect(useUIStore.getState().closeTab('a', 'a')).toBeNull();
    expect(tabs()).toEqual([]);
  });
});
