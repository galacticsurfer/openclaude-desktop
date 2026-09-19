import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react';
import { cn } from '@/lib/cn';

interface Props extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> {
  /** Required: this is the accessible name and the tooltip. */
  label: string;
  children: ReactNode;
  size?: 'sm' | 'md';
  active?: boolean;
  danger?: boolean;
}

/**
 * An icon-only button. `label` is mandatory so no control in the app can ship
 * without an accessible name.
 */
export const IconButton = forwardRef<HTMLButtonElement, Props>(function IconButton(
  { label, children, size = 'md', active, danger, className, ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type="button"
      aria-label={label}
      title={label}
      aria-pressed={active}
      className={cn(
        'inline-flex shrink-0 items-center justify-center rounded transition-colors',
        size === 'sm' ? 'size-6' : 'size-8',
        active ? 'bg-accent-soft text-accent' : 'text-ink-faint hover:bg-sunken hover:text-ink',
        danger && 'hover:bg-danger-soft hover:text-danger',
        'disabled:pointer-events-none disabled:opacity-40',
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
});
