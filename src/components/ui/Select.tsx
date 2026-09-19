import { forwardRef, type SelectHTMLAttributes } from 'react';
import { ChevronDown } from 'lucide-react';
import { cn } from '@/lib/cn';

/**
 * A native `<select>`, styled. Native is the right call here: it gets the
 * desktop's own popup behaviour, keyboard handling and screen-reader support
 * for free, which a div-based menu would have to reimplement.
 */
export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(
  function Select({ className, children, ...rest }, ref) {
    return (
      <div className="relative">
        <select
          ref={ref}
          className={cn(
            'h-9 w-full appearance-none rounded-md border border-line-strong bg-surface',
            'pl-3 pr-8 text-[14px] text-ink transition-colors focus:border-accent',
            'disabled:cursor-not-allowed disabled:opacity-60',
            className,
          )}
          {...rest}
        >
          {children}
        </select>
        <ChevronDown
          size={14}
          aria-hidden
          className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-ink-faint"
        />
      </div>
    );
  },
);
