import { useState } from 'react';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import { useUIStore, type ConfirmRequest } from '@/stores/useUIStore';

export function ConfirmDialog({ request }: { request: ConfirmRequest }) {
  const closeOverlay = useUIStore((s) => s.closeOverlay);
  const [busy, setBusy] = useState(false);

  return (
    <Dialog
      open
      onClose={closeOverlay}
      title={request.title}
      size="sm"
      footer={
        <>
          <Button variant="ghost" onClick={closeOverlay}>
            Cancel
          </Button>
          <Button
            variant={request.destructive ? 'danger' : 'primary'}
            loading={busy}
            data-autofocus
            onClick={async () => {
              setBusy(true);
              try {
                await request.onConfirm();
                closeOverlay();
              } finally {
                setBusy(false);
              }
            }}
          >
            {request.confirmLabel}
          </Button>
        </>
      }
    >
      <p className="px-5 py-4 text-[13.5px] leading-relaxed text-ink-soft">{request.body}</p>
    </Dialog>
  );
}
