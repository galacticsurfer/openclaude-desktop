import { create } from 'zustand';
import * as api from '@/services/api';
import type { CredentialStatus, ModelInfo, Settings, ThemePreference } from '@/types';

interface SettingsState {
  settings: Settings | null;
  models: ModelInfo[];
  modelsStale: boolean;
  credentials: CredentialStatus | null;
  loading: boolean;

  load: () => Promise<void>;
  set: <K extends keyof Settings>(key: K, value: Settings[K]) => Promise<void>;
  setMany: (values: Partial<Settings>) => Promise<void>;
  refreshModels: (force?: boolean) => Promise<void>;
  refreshCredentials: () => Promise<void>;

  /** Convenience reader with a default, for use before `load` resolves. */
  get: <K extends keyof Settings>(key: K, fallback: Settings[K]) => Settings[K];
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: null,
  models: [],
  modelsStale: false,
  credentials: null,
  loading: true,

  async load() {
    const [settings, credentials] = await Promise.all([
      api.getSettings(),
      api.credentialStatus(),
    ]);
    set({ settings, credentials, loading: false });

    // The theme is mirrored into localStorage purely so the inline script in
    // index.html can apply it before first paint next launch.
    try {
      localStorage.setItem('openclaude.theme', settings['appearance.theme']);
    } catch {
      /* private mode or blocked storage — the in-app theme still works */
    }
  },

  async set(key, value) {
    // Optimistic: settings changes should feel instant.
    const current = get().settings;
    if (current) set({ settings: { ...current, [key]: value } });

    if (key === 'appearance.theme') {
      try {
        localStorage.setItem('openclaude.theme', value as ThemePreference);
      } catch {
        /* ignore */
      }
    }

    try {
      await api.setSetting(key, value);
    } catch (err) {
      // Roll back so the UI never claims a change that did not persist.
      if (current) set({ settings: current });
      throw err;
    }
  },

  async setMany(values) {
    const current = get().settings;
    if (current) set({ settings: { ...current, ...values } });
    try {
      await api.setSettings(values);
    } catch (err) {
      if (current) set({ settings: current });
      throw err;
    }
  },

  async refreshModels(force = false) {
    const result = await api.listModels(force);
    set({ models: result.models, modelsStale: result.stale });

    // Seed the default model the first time we learn what is available.
    const settings = get().settings;
    if (settings && !settings['claude.defaultModel'] && result.models.length > 0) {
      const preferred =
        result.models.find((m) => /sonnet/i.test(m.id)) ?? result.models[0];
      if (preferred) void get().set('claude.defaultModel', preferred.id);
    }
  },

  async refreshCredentials() {
    set({ credentials: await api.credentialStatus() });
  },

  get(key, fallback) {
    const s = get().settings;
    return s ? (s[key] ?? fallback) : fallback;
  },
}));

/** The model a new conversation should start on. */
export function defaultModel(): string {
  const { settings, models } = useSettingsStore.getState();
  return (
    settings?.['claude.defaultModel'] ??
    models.find((m) => /sonnet/i.test(m.id))?.id ??
    models[0]?.id ??
    'claude-sonnet-4-5'
  );
}
