import { useEffect } from 'react';
import { AlertCircle } from 'lucide-react';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { Select } from '@/components/ui/Select';

interface Props {
  value: string;
  onChange: (model: string) => void;
  disabled?: boolean;
}

/**
 * Model picker.
 *
 * These are Claude Code's own aliases rather than pinned ids, so each one
 * follows whatever model is current — a hardcoded list would go stale. A full
 * model name can be typed instead, and whatever the conversation already uses
 * is always offered, so an old conversation never loses its setting.
 */
export function ModelSelector({ value, onChange, disabled }: Props) {
  const { models, modelsStale, refreshModels } = useSettingsStore();

  useEffect(() => {
    if (models.length === 0) void refreshModels();
  }, [models.length, refreshModels]);

  const known = models.some((m) => m.id === value);
  const options = known
    ? models
    : [{ id: value, displayName: value, fromFallback: false }, ...models];

  const CUSTOM = '__custom__';

  return (
    <div className="flex items-center gap-1.5">
      <Select
        value={value}
        onChange={(e) => {
          if (e.target.value !== CUSTOM) {
            onChange(e.target.value);
            return;
          }
          // Full names like `claude-fable-5` are valid too; the CLI accepts
          // either. Prompt rather than hide the capability.
          const name = window.prompt(
            'Model name (an alias such as "opus", or a full name such as "claude-fable-5")',
            value,
          );
          if (name && name.trim()) onChange(name.trim());
        }}
        disabled={disabled}
        aria-label="Model"
        className="h-7 min-w-[170px] max-w-[240px] border-transparent bg-transparent pl-2 pr-7 text-[13px] text-ink-soft hover:bg-sunken"
      >
        {options.map((m) => (
          <option key={m.id} value={m.id}>
            {m.displayName}
          </option>
        ))}
        <option value={CUSTOM}>Other model…</option>
      </Select>
      {modelsStale && (
        <span title="Model list may be incomplete." className="text-warn">
          <AlertCircle size={13} aria-label="Model list may be incomplete" />
        </span>
      )}
    </div>
  );
}
