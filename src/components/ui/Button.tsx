import { forwardRef, type ButtonHTMLAttributes } from 'react';
import { cn } from '@/lib/cn';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
type Size = 'sm' | 'md' | 'lg';

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  loading?: boolean;
}

const VARIANTS: Record<Variant, string> = {
  primary:
    'bg-accent text-on-accent hover:bg-accent-hover disabled:bg-accent/50 shadow-subtle',
  secondary:
    'bg-surface text-ink border border-line-strong hover:bg-sunken disabled:opacity-50',
  ghost: 'text-ink-soft hover:bg-sunken hover:text-ink disabled:opacity-40',
  danger: 'bg-danger text-white hover:brightness-110 disabled:opacity-50',
};

const SIZES: Record<Size, string> = {
  sm: 'h-7 px-2.5 text-[13px] gap-1.5 rounded',
  md: 'h-9 px-3.5 text-[14px] gap-2 rounded-md',
  lg: 'h-11 px-5 text-[15px] gap-2 rounded-md',
};

export const Button = forwardRef<HTMLButtonElement, Props>(function Button(
  { variant = 'secondary', size = 'md', loading, className, children, disabled, ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      className={cn(
        'inline-flex select-none items-center justify-center font-medium transition-colors',
        'disabled:cursor-not-allowed',
        VARIANTS[variant],
        SIZES[size],
        className,
      )}
      {...rest}
    >
      {loading && (
        <span
          aria-hidden
          className="size-3.5 animate-spin rounded-full border-2 border-current border-r-transparent opacity-70"
        />
      )}
      {children}
    </button>
  );
});
