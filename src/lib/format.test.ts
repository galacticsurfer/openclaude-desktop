import { describe, expect, it, vi, afterEach } from 'vitest';
import {
  dateGroup, estimateCost, formatBytes, formatCost, formatRelative, formatTokens,
} from './format';

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

describe('estimateCost', () => {
  const pricing = {
    models: {
      'claude-sonnet-4-5': { input: 3, output: 15 },
      'claude-opus-4-5': { input: 5, output: 25 },
    },
  };

  it('prices a known model per million tokens', () => {
    // 1M in at $3 + 0.5M out at $15 = 3 + 7.5
    expect(estimateCost('claude-sonnet-4-5', 1_000_000, 500_000, pricing)).toBeCloseTo(10.5);
  });

  it('matches a dated model id against its family prefix', () => {
    const dated = estimateCost('claude-sonnet-4-5-20260101', 1_000_000, 0, pricing);
    expect(dated).toBeCloseTo(3);
  });

  it('returns null rather than guessing for an unknown model', () => {
    expect(estimateCost('some-other-model', 1000, 1000, pricing)).toBeNull();
    expect(estimateCost('claude-sonnet-4-5', 1000, 1000, undefined)).toBeNull();
  });

  it('formats small amounts with enough precision to be meaningful', () => {
    expect(formatCost(0.0042, 'USD')).toMatch(/0\.004/);
    expect(formatCost(12.5, 'USD')).toMatch(/12\.50/);
  });
});
