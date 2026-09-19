import { describe, expect, it } from 'vitest';
import { AppError } from './ipc';

describe('AppError.from', () => {
  it('preserves a structured payload from the Rust side', () => {
    const err = AppError.from({
      kind: 'rate_limit_error',
      message: 'Rate limit reached.',
      retryable: true,
      detail: {
        kind: 'rate_limit_error',
        message: 'Rate limit reached.',
        status: 429,
        requestId: 'req_1',
        retryable: true,
        retryAfterSecs: 30,
      },
    });

    expect(err.kind).toBe('rate_limit_error');
    expect(err.retryable).toBe(true);
    expect(err.detail?.status).toBe(429);
    expect(err.detail?.retryAfterSecs).toBe(30);
  });

  it('normalises a bare string rejection', () => {
    const err = AppError.from('command not found');
    expect(err.kind).toBe('unknown');
    expect(err.message).toBe('command not found');
    expect(err.retryable).toBe(false);
  });

  it('normalises a thrown Error', () => {
    expect(AppError.from(new Error('boom')).message).toBe('boom');
  });

  it('always produces a message, even for nonsense input', () => {
    expect(AppError.from(null).message).toBeTruthy();
    expect(AppError.from(undefined).message).toBeTruthy();
  });

  it('is idempotent', () => {
    const a = AppError.from('x');
    expect(AppError.from(a)).toBe(a);
  });

  it('classifies credential and connectivity failures for the UI', () => {
    expect(
      AppError.from({ kind: 'missing_credentials', message: 'no key', retryable: false })
        .isMissingCredentials,
    ).toBe(true);
    expect(AppError.from({ kind: 'offline', message: 'x', retryable: true }).isOffline).toBe(true);
  });
});
