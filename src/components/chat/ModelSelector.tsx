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
 * The list comes from the provider, never from a hardcoded array: ids are
 * stored, display names are only shown. A model the conversation already uses
 * is always offered even if it has since disappeared from the account's list,
 * so an old conversation never loses its setting.
 */
export function ModelSelector({ value, onChange, disabled }: Props) {
  const { models, modelsStale, refreshModels } = useSettingsStore();

  useEffect(() => {
    if (models.length === 0) void refreshModels();
  }, [models.length, refreshModels]);

  const known = models.some((m) => m.id === value);
  const options = known ? models : [{ id: value, displayName: value, fromFallback: false }, ...models];

  return (
    <div className="flex items-center gap-1.5">
      <Select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        aria-label="Model"
        className="h-7 min-w-[170px] max-w-[240px] border-transparent bg-transparent pl-2 pr-7 text-[13px] text-ink-soft hover:bg-sunken"
      >
        {options.map((m) => (
          <option key={m.id} value={m.id}>
            {m.displayName}
          </option>
        ))}
      </Select>
      {modelsStale && (
        <span
          title="This list could not be refreshed from the API, so it may be incomplete."
          className="text-warn"
        >
          <AlertCircle size={13} aria-label="Model list may be out of date" />
        </span>
      )}
    </div>
  );
}
