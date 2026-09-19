import { create } from 'zustand';

export type Overlay =
  | { kind: 'none' }
  | { kind: 'search' }
  | { kind: 'commandPalette' }
  | { kind: 'settings'; section?: string }
  | { kind: 'shortcuts' }
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

  toggleSidebar: () => void;
  setSidebarCollapsed: (v: boolean) => void;
  openOverlay: (o: Overlay) => void;
  closeOverlay: () => void;
  confirm: (req: ConfirmRequest) => void;
  toast: (kind: Toast['kind'], message: string, action?: Toast['action']) => void;
  dismissToast: (id: number) => void;
  setActiveProject: (id: string | null) => void;
  setScope: (s: UIState['scope']) => void;
}

let toastSeq = 0;

export const useUIStore = create<UIState>((set, get) => ({
  sidebarCollapsed: false,
  overlay: { kind: 'none' },
  toasts: [],
  activeProjectId: null,
  scope: 'active',

  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  setSidebarCollapsed: (v) => set({ sidebarCollapsed: v }),

  openOverlay: (overlay) => set({ overlay }),
  closeOverlay: () => set({ overlay: { kind: 'none' } }),

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
}));
