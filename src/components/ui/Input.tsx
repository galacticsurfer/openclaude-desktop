import { forwardRef, type InputHTMLAttributes, type TextareaHTMLAttributes } from 'react';
import { cn } from '@/lib/cn';

const base =
  'w-full rounded-md border border-line-strong bg-surface px-3 text-ink placeholder:text-ink-faint ' +
  'transition-colors focus:border-accent disabled:opacity-60 disabled:cursor-not-allowed';

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...rest }, ref) {
    return <input ref={ref} className={cn(base, 'h-9 text-[14px]', className)} {...rest} />;
  },
);

export const Textarea = forwardRef<
  HTMLTextAreaElement,
  TextareaHTMLAttributes<HTMLTextAreaElement>
>(function Textarea({ className, ...rest }, ref) {
  return (
    <textarea
      ref={ref}
      className={cn(base, 'resize-y py-2 text-[14px] leading-relaxed scroll-thin', className)}
      {...rest}
    />
  );
});

interface FieldProps {
  label: string;
  hint?: string;
  htmlFor?: string;
  children: React.ReactNode;
  className?: string;
}

export function Field({ label, hint, htmlFor, children, className }: FieldProps) {
  return (
    <div className={cn('space-y-1.5', className)}>
      <label htmlFor={htmlFor} className="block text-[13px] font-medium text-ink">
        {label}
      </label>
      {children}
      {hint && <p className="text-[12px] leading-snug text-ink-faint">{hint}</p>}
    </div>
  );
}
