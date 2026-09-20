import { create } from 'zustand';
import * as api from '@/services/api';
import type { ClaudeCodeStatus, ModelInfo, Settings, ThemePreference } from '@/types';

interface SettingsState {
  settings: Settings | null;
  models: ModelInfo[];
  modelsStale: boolean;
  /** What the CLI says it is currently using. */
  currentModel: string | null;
  cliEffort: string | null;
  claudeCode: ClaudeCodeStatus | null;
  loading: boolean;

  load: () => Promise<void>;
  set: <K extends keyof Settings>(key: K, value: Settings[K]) => Promise<void>;
  setMany: (values: Partial<Settings>) => Promise<void>;
  refreshModels: (force?: boolean) => Promise<void>;
  refreshClaudeCode: () => Promise<void>;

  /** Convenience reader with a default, for use before `load` resolves. */
  get: <K extends keyof Settings>(key: K, fallback: Settings[K]) => Settings[K];
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: null,
  models: [],
  modelsStale: false,
  currentModel: null,
  cliEffort: null,
  claudeCode: null,
  loading: true,

  async load() {
    const [settings, claudeCode] = await Promise.all([
      api.getSettings(),
      api.claudeCodeStatus().catch(() => null),
    ]);
    set({ settings, claudeCode, loading: false });

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
    set({
      models: result.models,
      modelsStale: result.stale,
      currentModel: result.current,
      cliEffort: result.effort,
    });

    // Seed the default model the first time we learn what is available.
    const settings = get().settings;
    if (settings && !settings['claude.defaultModel'] && result.models.length > 0) {
      const preferred =
        result.models.find((m) => /sonnet/i.test(m.id)) ?? result.models[0];
      if (preferred) void get().set('claude.defaultModel', preferred.id);
    }
  },

  async refreshClaudeCode() {
    set({ claudeCode: await api.claudeCodeStatus().catch(() => null) });
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
