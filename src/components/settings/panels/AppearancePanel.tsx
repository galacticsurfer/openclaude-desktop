import { Monitor, Moon, Sun } from 'lucide-react';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { Switch } from '@/components/ui/Switch';
import { Group, Row } from '../SettingsDialog';
import { cn } from '@/lib/cn';
import type { ThemePreference } from '@/types';

const THEMES: Array<{ id: ThemePreference; label: string; icon: typeof Sun }> = [
  { id: 'system', label: 'System', icon: Monitor },
  { id: 'light', label: 'Light', icon: Sun },
  { id: 'dark', label: 'Dark', icon: Moon },
];

export function AppearancePanel() {
  const { settings, set } = useSettingsStore();
  if (!settings) return null;

  return (
    <>
      <Group title="Theme">
        <div className="flex gap-2 py-2">
          {THEMES.map((t) => {
            const Icon = t.icon;
            const active = settings['appearance.theme'] === t.id;
            return (
              <button
                key={t.id}
                type="button"
                onClick={() => void set('appearance.theme', t.id)}
                aria-pressed={active}
                className={cn(
                  'flex flex-1 flex-col items-center gap-1.5 rounded-lg border px-3 py-3 transition-colors',
                  active
                    ? 'border-accent bg-accent-soft text-accent'
                    : 'border-line text-ink-soft hover:border-line-strong hover:bg-sunken',
                )}
              >
                <Icon size={18} aria-hidden />
                <span className="text-[12.5px] font-medium">{t.label}</span>
              </button>
            );
          })}
        </div>
      </Group>

      <Group title="Text">
        <Row
          label="Font size"
          description={`Currently ${settings['appearance.fontSize']}px.`}
          control={
            <input
              type="range"
              min={12}
              max={20}
              step={1}
              value={settings['appearance.fontSize']}
              onChange={(e) => void set('appearance.fontSize', Number(e.target.value))}
              aria-label="Font size"
              className="w-full accent-[rgb(var(--c-accent))]"
            />
          }
        />
        <Switch
          checked={settings['appearance.compact']}
          onChange={(v) => void set('appearance.compact', v)}
          label="Compact mode"
          description="Tightens vertical spacing so more of the conversation fits on screen."
        />
      </Group>

      <Group title="Code">
        <Switch
          checked={settings['appearance.codeWrap']}
          onChange={(v) => void set('appearance.codeWrap', v)}
          label="Wrap long lines in code blocks"
          description="Individual blocks can still be toggled from their header."
        />
      </Group>

      <Group title="Accessibility">
        <Switch
          checked={settings['appearance.reducedMotion']}
          onChange={(v) => void set('appearance.reducedMotion', v)}
          label="Reduce motion"
          description="Disables animations and transitions. Your desktop's own reduced-motion setting is always respected regardless of this."
        />
      </Group>
    </>
  );
}
