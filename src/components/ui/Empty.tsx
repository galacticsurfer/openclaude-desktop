import type { LucideIcon } from 'lucide-react';
import type { ReactNode } from 'react';

interface Props {
  icon: LucideIcon;
  title: string;
  body?: string;
  action?: ReactNode;
}

export function Empty({ icon: Icon, title, body, action }: Props) {
  return (
    <div className="flex flex-col items-center justify-center px-6 py-12 text-center">
      <Icon size={28} className="mb-3 text-ink-faint/60" aria-hidden strokeWidth={1.5} />
      <p className="text-[14px] font-medium text-ink-soft">{title}</p>
      {body && <p className="mt-1 max-w-xs text-[13px] leading-snug text-ink-faint">{body}</p>}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
