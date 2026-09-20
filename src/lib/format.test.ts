import { describe, expect, it, vi, afterEach } from 'vitest';
import { dateGroup, formatBytes, formatRelative, formatTokens } from './format';

afterEach(() => vi.useRealTimers());

describe('formatBytes', () => {
  it('scales units and keeps the label short', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(1024 * 1024 * 3.5)).toBe('3.5 MB');
    // Large values drop the decimal rather than reading "12.0 MB".
    expect(formatBytes(1024 * 1024 * 12)).toBe('12 MB');
  });
});

describe('formatTokens', () => {
  it('abbreviates thousands and millions', () => {
    expect(formatTokens(42)).toBe('42');
    expect(formatTokens(1500)).toBe('1.5k');
    expect(formatTokens(48_000)).toBe('48k');
    expect(formatTokens(2_400_000)).toBe('2.4M');
  });
});

describe('formatRelative', () => {
  it('uses coarse units that stay short in the sidebar', () => {
    const now = new Date('2026-09-20T12:00:00Z').getTime();
    vi.useFakeTimers();
    vi.setSystemTime(now);

    expect(formatRelative(now - 30_000)).toBe('now');
    expect(formatRelative(now - 5 * 60_000)).toBe('5m');
    expect(formatRelative(now - 3 * 3_600_000)).toBe('3h');
    expect(formatRelative(now - 2 * 86_400_000)).toBe('2d');
    // Beyond a week it becomes a date rather than an ever-growing count.
    expect(formatRelative(now - 40 * 86_400_000)).toMatch(/\w/);
  });
});

describe('dateGroup', () => {
  it('buckets by local calendar day, not by elapsed hours', () => {
    vi.useFakeTimers();
    // 00:30 local — something from 23:00 "yesterday" is 1.5h ago but must
    // still be labelled Yesterday, not Today.
    const now = new Date(2026, 8, 20, 0, 30);
    vi.setSystemTime(now);

    expect(dateGroup(new Date(2026, 8, 20, 0, 10).getTime())).toBe('Today');
    expect(dateGroup(new Date(2026, 8, 19, 23, 0).getTime())).toBe('Yesterday');
    expect(dateGroup(new Date(2026, 8, 17, 9, 0).getTime())).toBe('Previous 7 days');
    expect(dateGroup(new Date(2026, 8, 1, 9, 0).getTime())).toBe('Previous 30 days');
    expect(dateGroup(new Date(2025, 2, 1, 9, 0).getTime())).toBe('2025');
  });
});
