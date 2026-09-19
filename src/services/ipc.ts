import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { AppErrorPayload } from '@/types';

/**
 * An error that crossed the IPC boundary.
 *
 * The Rust side always rejects with a structured payload, but a genuinely
 * unexpected failure (a missing command, a serialisation bug) arrives as a
 * bare string — `from` normalises both into the same shape so the UI only
 * ever handles one.
 */
export class AppError extends Error {
  readonly kind: string;
  readonly retryable: boolean;
  readonly detail: AppErrorPayload['detail'];

  constructor(payload: AppErrorPayload) {
    super(payload.message);
    this.name = 'AppError';
    this.kind = payload.kind;
    this.retryable = payload.retryable;
    this.detail = payload.detail;
  }

  static from(err: unknown): AppError {
    if (err instanceof AppError) return err;

    if (err && typeof err === 'object' && 'kind' in err && 'message' in err) {
      return new AppError(err as AppErrorPayload);
    }

    const message =
      typeof err === 'string'
        ? err
        : err instanceof Error
          ? err.message
          : 'Something went wrong.';
    return new AppError({ kind: 'unknown', message, retryable: false });
  }

  /** True when the failure is "no API key yet" rather than a real fault. */
  get isMissingCredentials(): boolean {
    return this.kind === 'missing_credentials';
  }

  get isOffline(): boolean {
    return this.kind === 'offline' || this.kind === 'network';
  }
}

/** Typed `invoke` that always rejects with an `AppError`. */
export async function invoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await tauriInvoke<T>(command, args);
  } catch (err) {
    throw AppError.from(err);
  }
}

/**
 * Run an IPC call and return `fallback` instead of throwing.
 * For calls whose failure should degrade the UI rather than break it.
 */
export async function tryInvoke<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  fallback: T,
): Promise<T> {
  try {
    return await tauriInvoke<T>(command, args);
  } catch (err) {
    console.warn(`[ipc] ${command} failed:`, AppError.from(err).message);
    return fallback;
  }
}
