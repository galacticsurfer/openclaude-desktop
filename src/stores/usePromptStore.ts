import { create } from 'zustand';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useUIStore } from './useUIStore';
import type { Prompt } from '@/types';

interface PromptState {
  prompts: Prompt[];
  loaded: boolean;
  load: () => Promise<void>;
  save: (title: string, body: string, id?: string) => Promise<boolean>;
  remove: (id: string) => Promise<void>;
  /** Record a use, so the picker can lead with what is actually used. */
  markUsed: (id: string) => Promise<void>;
}

export const usePromptStore = create<PromptState>((set, get) => ({
  prompts: [],
  loaded: false,

  async load() {
    try {
      set({ prompts: await api.listPrompts(), loaded: true });
    } catch (err) {
      useUIStore.getState().toast('error', AppError.from(err).message);
    }
  },

  async save(title, body, id) {
    try {
      if (id) await api.updatePrompt(id, title, body);
      else await api.createPrompt(title, body);
      await get().load();
      return true;
    } catch (err) {
      useUIStore.getState().toast('error', AppError.from(err).message);
      return false;
    }
  },

  async remove(id) {
    try {
      await api.deletePrompt(id);
      await get().load();
    } catch (err) {
      useUIStore.getState().toast('error', AppError.from(err).message);
    }
  },

  async markUsed(id) {
    // Ordering only; a failure here must not block inserting the text.
    try {
      await api.markPromptUsed(id);
      await get().load();
    } catch {
      /* ignored */
    }
  },
}));
