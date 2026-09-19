import { cn } from '@/lib/cn';

interface Props {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
}

export function Switch({ checked, onChange, label, description, disabled }: Props) {
  return (
    <label
      className={cn(
        'flex cursor-pointer items-start justify-between gap-6 py-2',
        disabled && 'cursor-not-allowed opacity-60',
      )}
    >
      <span className="min-w-0">
        <span className="block text-[14px] text-ink">{label}</span>
        {description && (
          <span className="mt-0.5 block text-[12.5px] leading-snug text-ink-faint">
            {description}
          </span>
        )}
      </span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          'relative mt-0.5 h-5 w-9 shrink-0 rounded-full transition-colors',
          checked ? 'bg-accent' : 'bg-line-strong',
          disabled && 'pointer-events-none',
        )}
      >
        <span
          className={cn(
            'absolute top-0.5 size-4 rounded-full bg-white shadow-subtle transition-transform',
            checked ? 'translate-x-[18px]' : 'translate-x-0.5',
          )}
        />
      </button>
    </label>
  );
}
