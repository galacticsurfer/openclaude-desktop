import { useEffect, useState } from 'react';
import { FileText, FileCode, FileJson, Image as ImageIcon, FileType, X } from 'lucide-react';
import type { Attachment } from '@/types';
import { formatBytes } from '@/lib/format';
import * as api from '@/services/api';
import { cn } from '@/lib/cn';

interface Props {
  attachment: Attachment;
  onRemove?: () => void;
  readOnly?: boolean;
}

function iconFor(a: Attachment) {
  if (a.kind === 'image') return ImageIcon;
  if (a.mimeType === 'application/pdf') return FileType;
  if (/json/.test(a.mimeType) || a.filename.endsWith('.json')) return FileJson;
  if (a.kind === 'text' && /\.(ts|tsx|js|jsx|py|rs|go|rb|java|c|cpp|h|sh|sql)$/.test(a.filename)) {
    return FileCode;
  }
  return FileText;
}

export function AttachmentChip({ attachment: a, onRemove, readOnly }: Props) {
  const Icon = iconFor(a);
  const [thumb, setThumb] = useState<string | null>(null);

  useEffect(() => {
    if (a.kind !== 'image') return;
    let cancelled = false;
    void api
      .readAttachmentDataUrl(a.id)
      .then((url) => {
        if (!cancelled) setThumb(url);
      })
      .catch(() => {
        /* fall back to the icon */
      });
    return () => {
      cancelled = true;
    };
  }, [a.id, a.kind]);

  return (
    <div
      className={cn(
        'group/chip flex max-w-[220px] items-center gap-2 rounded-md border border-line bg-surface py-1 pl-1.5 pr-2',
        readOnly ? 'bg-sunken/60' : 'shadow-subtle',
      )}
    >
      {thumb ? (
        <img
          src={thumb}
          alt=""
          className="size-7 shrink-0 rounded object-cover"
        />
      ) : (
        <span className="flex size-7 shrink-0 items-center justify-center rounded bg-sunken text-ink-faint">
          <Icon size={14} aria-hidden />
        </span>
      )}

      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12.5px] leading-tight text-ink" title={a.filename}>
          {a.filename}
        </span>
        <span className="block text-[11px] leading-tight text-ink-faint">
          {formatBytes(a.sizeBytes)}
        </span>
      </span>

      {!readOnly && onRemove && (
        <button
          type="button"
          onClick={onRemove}
          aria-label={`Remove ${a.filename}`}
          className="shrink-0 rounded p-0.5 text-ink-faint opacity-0 transition-opacity hover:text-danger focus-visible:opacity-100 group-hover/chip:opacity-100"
        >
          <X size={13} />
        </button>
      )}
    </div>
  );
}
