import { Dialog } from '@/components/ui/Dialog';
import { useUIStore } from '@/stores/useUIStore';
import { ShortcutsPanel } from '@/components/settings/panels/ShortcutsPanel';

export function ShortcutsDialog() {
  const closeOverlay = useUIStore((s) => s.closeOverlay);
  return (
    <Dialog open onClose={closeOverlay} title="Keyboard shortcuts" size="md">
      <div className="p-5">
        <ShortcutsPanel />
      </div>
    </Dialog>
  );
}
