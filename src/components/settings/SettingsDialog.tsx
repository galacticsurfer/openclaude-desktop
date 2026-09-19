import { useEffect, useState } from 'react';
import {
  Cpu, Database, Eye, Keyboard, Lock, Plug, Settings2, Sliders,
} from 'lucide-react';
import { Dialog } from '@/components/ui/Dialog';
import { useUIStore } from '@/stores/useUIStore';
import { cn } from '@/lib/cn';
import { GeneralPanel } from './panels/GeneralPanel';
import { AppearancePanel } from './panels/AppearancePanel';
import { ClaudePanel } from './panels/ClaudePanel';
import { McpPanel } from './panels/McpPanel';
import { NotificationsPanel } from './panels/NotificationsPanel';
import { PrivacyPanel } from './panels/PrivacyPanel';
import { AdvancedPanel } from './panels/AdvancedPanel';
import { ShortcutsPanel } from './panels/ShortcutsPanel';

const SECTIONS = [
  { id: 'general', label: 'General', icon: Settings2, Panel: GeneralPanel },
  { id: 'appearance', label: 'Appearance', icon: Eye, Panel: AppearancePanel },
  { id: 'claude', label: 'Claude', icon: Cpu, Panel: ClaudePanel },
  { id: 'mcp', label: 'MCP servers', icon: Plug, Panel: McpPanel },
  { id: 'notifications', label: 'Notifications', icon: Sliders, Panel: NotificationsPanel },
  { id: 'shortcuts', label: 'Shortcuts', icon: Keyboard, Panel: ShortcutsPanel },
  { id: 'privacy', label: 'Privacy', icon: Lock, Panel: PrivacyPanel },
  { id: 'advanced', label: 'Advanced', icon: Database, Panel: AdvancedPanel },
] as const;

export function SettingsDialog({ section }: { section?: string }) {
  const closeOverlay = useUIStore((s) => s.closeOverlay);
  const [active, setActive] = useState(section ?? 'general');

  useEffect(() => {
    if (section) setActive(section);
  }, [section]);

  const current = SECTIONS.find((s) => s.id === active) ?? SECTIONS[0];
  const Panel = current.Panel;

  return (
    <Dialog open onClose={closeOverlay} size="xl" title="Settings" className="h-[640px]">
      <div className="flex h-full min-h-0">
        <nav
          className="w-48 shrink-0 overflow-y-auto scroll-thin border-r border-line bg-sunken/40 p-2"
          aria-label="Settings sections"
        >
          {SECTIONS.map((s) => {
            const Icon = s.icon;
            return (
              <button
                key={s.id}
                type="button"
                onClick={() => setActive(s.id)}
                aria-current={active === s.id ? 'page' : undefined}
                className={cn(
                  'flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-[13.5px] transition-colors',
                  active === s.id
                    ? 'bg-surface font-medium text-ink shadow-subtle'
                    : 'text-ink-soft hover:bg-surface/60 hover:text-ink',
                )}
              >
                <Icon size={15} className="shrink-0 text-ink-faint" aria-hidden />
                {s.label}
              </button>
            );
          })}
        </nav>

        <div className="min-w-0 flex-1 overflow-y-auto scroll-thin px-6 py-5">
          <h3 className="mb-4 text-[15px] font-semibold tracking-[-0.01em] text-ink">
            {current.label}
          </h3>
          <Panel />
        </div>
      </div>
    </Dialog>
  );
}

/** Shared layout for a labelled group of controls. */
export function Group({
  title, description, children,
}: {
  title?: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="mb-7">
      {title && <h4 className="mb-1 text-[13px] font-semibold text-ink">{title}</h4>}
      {description && (
        <p className="mb-3 max-w-prose text-[12.5px] leading-snug text-ink-faint">{description}</p>
      )}
      <div className="divide-y divide-line">{children}</div>
    </section>
  );
}

export function Row({
  label, description, control,
}: {
  label: string;
  description?: string;
  control: React.ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-6 py-2.5">
      <div className="min-w-0">
        <p className="text-[14px] text-ink">{label}</p>
        {description && (
          <p className="mt-0.5 max-w-prose text-[12.5px] leading-snug text-ink-faint">
            {description}
          </p>
        )}
      </div>
      <div className="w-48 shrink-0">{control}</div>
    </div>
  );
}
