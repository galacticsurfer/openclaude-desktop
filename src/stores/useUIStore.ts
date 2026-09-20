import { create } from 'zustand';

export type Overlay =
  | { kind: 'none' }
  | { kind: 'search' }
  | { kind: 'commandPalette' }
  | { kind: 'settings'; section?: string }
  | { kind: 'shortcuts' }
  | { kind: 'prompts' }
  | { kind: 'newProject' }
  | { kind: 'editProject'; projectId: string }
  | { kind: 'conversationInfo'; conversationId: string }
  | { kind: 'confirm'; request: ConfirmRequest };

export interface ConfirmRequest {
  title: string;
  body: string;
  confirmLabel: string;
  destructive?: boolean;
  onConfirm: () => void | Promise<void>;
}

export interface Toast {
  id: number;
  kind: 'info' | 'error' | 'success';
  message: string;
  /** Optional inline action, e.g. "Undo". */
  action?: { label: string; run: () => void };
}

interface UIState {
  sidebarCollapsed: boolean;
  overlay: Overlay;
  toasts: Toast[];
  /** Which project the sidebar is filtered to, if any. */
  activeProjectId: string | null;
  scope: 'active' | 'archived' | 'trash';
  /** A file drag is in progress over the window. */
  dragActive: boolean;

  toggleSidebar: () => void;
  setSidebarCollapsed: (v: boolean) => void;
  openOverlay: (o: Overlay) => void;
  closeOverlay: () => void;

  /**
   * Conversation ids with an open tab, in strip order.
   *
   * The *active* tab is not stored here — that is the conversation store's
   * `currentId`, and duplicating it would let the two disagree. Tabs are
   * deliberately session-only: reopening yesterday's tab strip is more
   * surprising than useful.
   */
  /**
   * A code block or diagram pulled out of the transcript into the side
   * panel. Not an executable artifact: this app renders what Claude wrote,
   * it does not run it.
   */
  artifact: { title: string; code: string; language?: string } | null;
  openArtifact: (a: { title: string; code: string; language?: string }) => void;
  closeArtifact: () => void;

  /** The embedded shell panel. Closing it kills the shell. */
  terminalOpen: boolean;
  toggleTerminal: () => void;
  closeTerminal: () => void;

  tabs: string[];
  /** Show `id` in the active tab, or focus the tab already showing it. */
  openInTab: (id: string, activeId: string | null) => void;
  /** Add a tab next to the active one without disturbing it. */
  openInNewTab: (id: string, activeId: string | null) => void;
  /** Close a tab; returns the id to show next, or null if none is left. */
  closeTab: (id: string, activeId: string | null) => string | null;
  /**
   * Text an overlay wants dropped into the composer.
   *
   * A one-shot handoff rather than a direct call: the composer owns its own
   * draft state and is not mounted while a modal has focus, so the value is
   * parked here and claimed on the next render.
   */
  composerInsert: string | null;
  insertIntoComposer: (text: string) => void;
  claimComposerInsert: () => string | null;
  confirm: (req: ConfirmRequest) => void;
  toast: (kind: Toast['kind'], message: string, action?: Toast['action']) => void;
  dismissToast: (id: number) => void;
  setActiveProject: (id: string | null) => void;
  setScope: (s: UIState['scope']) => void;
  setDragActive: (v: boolean) => void;
}

let toastSeq = 0;

export const useUIStore = create<UIState>((set, get) => ({
  sidebarCollapsed: false,
  overlay: { kind: 'none' },
  toasts: [],
  activeProjectId: null,
  scope: 'active',
  dragActive: false,

  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  setSidebarCollapsed: (v) => set({ sidebarCollapsed: v }),

  openOverlay: (overlay) => set({ overlay }),
  closeOverlay: () => set({ overlay: { kind: 'none' } }),

  artifact: null,
  openArtifact: (artifact) => set({ artifact }),
  closeArtifact: () => set({ artifact: null }),

  terminalOpen: false,
  toggleTerminal: () => set((s) => ({ terminalOpen: !s.terminalOpen })),
  closeTerminal: () => set({ terminalOpen: false }),

  tabs: [],

  openInTab: (id, activeId) =>
    set((s) => {
      if (s.tabs.includes(id)) return s;
      const at = activeId === null ? -1 : s.tabs.indexOf(activeId);
      // Replace what the active tab was showing, like following a link in
      // the same tab; with no active tab there is nothing to replace.
      if (at === -1) return { tabs: [...s.tabs, id] };
      const tabs = [...s.tabs];
      tabs[at] = id;
      return { tabs };
    }),

  openInNewTab: (id, activeId) =>
    set((s) => {
      if (s.tabs.includes(id)) return s;
      const at = activeId === null ? -1 : s.tabs.indexOf(activeId);
      const tabs = [...s.tabs];
      tabs.splice(at === -1 ? tabs.length : at + 1, 0, id);
      return { tabs };
    }),

  closeTab: (id, activeId) => {
    const tabs = get().tabs;
    const at = tabs.indexOf(id);
    if (at === -1) return activeId;
    const next = tabs.filter((t) => t !== id);
    set({ tabs: next });
    // Closing an inactive tab must not move the user.
    if (id !== activeId) return activeId;
    // Otherwise fall to the tab on the right, or the one on the left.
    return next[at] ?? next[at - 1] ?? null;
  },

  composerInsert: null,
  insertIntoComposer: (text) => set({ composerInsert: text }),
  claimComposerInsert: () => {
    const text = get().composerInsert;
    if (text !== null) set({ composerInsert: null });
    return text;
  },

  confirm: (request) => set({ overlay: { kind: 'confirm', request } }),

  toast: (kind, message, action) => {
    const id = ++toastSeq;
    set((s) => ({ toasts: [...s.toasts, { id, kind, message, action }] }));
    // Errors linger; anything else clears itself.
    const ttl = kind === 'error' ? 8000 : 3500;
    window.setTimeout(() => get().dismissToast(id), ttl);
  },

  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),

  setActiveProject: (activeProjectId) => set({ activeProjectId }),
  setScope: (scope) => set({ scope }),
  setDragActive: (dragActive) => set({ dragActive }),
}));
