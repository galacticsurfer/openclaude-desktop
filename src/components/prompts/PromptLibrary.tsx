import { useEffect, useState } from 'react';
import { BookMarked, Pencil, Plus, Trash2 } from 'lucide-react';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import { IconButton } from '@/components/ui/IconButton';
import { Empty } from '@/components/ui/Empty';
import { usePromptStore } from '@/stores/usePromptStore';
import { useUIStore } from '@/stores/useUIStore';

/**
 * Saved prompts: pick one to drop into the composer, or manage the list.
 *
 * Picking is the common case, so it is a single click on the row; editing
 * and deleting sit behind hover controls rather than a separate mode.
 */
export function PromptLibrary({ onInsert }: { onInsert: (body: string) => void }) {
  const { prompts, loaded, load, save, remove, markUsed } = usePromptStore();
  const closeOverlay = useUIStore((s) => s.closeOverlay);
  const confirm = useUIStore((s) => s.confirm);

  const [editingId, setEditingId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [title, setTitle] = useState('');
  const [body, setBody] = useState('');

  useEffect(() => {
    if (!loaded) void load();
  }, [loaded, load]);

  const editor = creating || editingId !== null;

  function startCreate() {
    setCreating(true);
    setEditingId(null);
    setTitle('');
    setBody('');
  }

  async function commit() {
    if (await save(title, body, editingId ?? undefined)) {
      setCreating(false);
      setEditingId(null);
    }
  }

  return (
    <Dialog
      open
      onClose={closeOverlay}
      title="Prompt library"
      description="Reusable message text. Pick one to drop it into the composer."
      size="lg"
      footer={
        editor ? (
          <>
            <Button
              variant="ghost"
              onClick={() => {
                setCreating(false);
                setEditingId(null);
              }}
            >
              Cancel
            </Button>
            <Button
              disabled={title.trim() === '' || body.trim() === ''}
              onClick={() => void commit()}
            >
              {editingId ? 'Save changes' : 'Add prompt'}
            </Button>
          </>
        ) : (
          <Button variant="secondary" onClick={startCreate}>
            <Plus size={14} /> New prompt
          </Button>
        )
      }
    >
      {editor ? (
        <div className="space-y-3">
          <label className="block">
            <span className="mb-1 block text-[13px] text-ink-soft">Name</span>
            <input
              value={title}
              autoFocus
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Summarise a document"
              className="h-8 w-full rounded border border-line bg-surface px-2 text-[13.5px] text-ink placeholder:text-ink-faint focus:border-line-strong"
            />
          </label>
          <label className="block">
            <span className="mb-1 block text-[13px] text-ink-soft">Prompt</span>
            <textarea
              value={body}
              rows={8}
              onChange={(e) => setBody(e.target.value)}
              placeholder="Summarise the following in five bullet points:"
              className="w-full resize-none rounded border border-line bg-surface px-2 py-1.5 text-[13.5px] leading-relaxed text-ink placeholder:text-ink-faint focus:border-line-strong scroll-thin"
            />
          </label>
        </div>
      ) : prompts.length === 0 ? (
        <Empty
          icon={BookMarked}
          title="No saved prompts yet"
          body="Keep the instructions you retype often, and drop them in with one click."
        />
      ) : (
        <ul className="-mx-1 max-h-[26rem] overflow-y-auto scroll-thin">
          {prompts.map((p) => (
            <li key={p.id} className="group/row relative">
              <button
                type="button"
                onClick={() => {
                  void markUsed(p.id);
                  onInsert(p.body);
                  closeOverlay();
                }}
                className="w-full rounded-md px-3 py-2 pr-16 text-left hover:bg-sunken"
              >
                <span className="block truncate text-[13.5px] font-medium text-ink">{p.title}</span>
                <span className="mt-0.5 block truncate text-[12.5px] text-ink-faint">{p.body}</span>
              </button>
              <div className="absolute right-2 top-1.5 flex gap-0.5 opacity-0 transition-opacity focus-within:opacity-100 group-hover/row:opacity-100">
                <IconButton
                  label={`Edit ${p.title}`}
                  size="sm"
                  onClick={() => {
                    setCreating(false);
                    setEditingId(p.id);
                    setTitle(p.title);
                    setBody(p.body);
                  }}
                >
                  <Pencil size={13} />
                </IconButton>
                <IconButton
                  label={`Delete ${p.title}`}
                  size="sm"
                  onClick={() =>
                    confirm({
                      title: `Delete "${p.title}"?`,
                      body: 'This cannot be undone.',
                      confirmLabel: 'Delete',
                      destructive: true,
                      onConfirm: () => void remove(p.id),
                    })
                  }
                >
                  <Trash2 size={13} />
                </IconButton>
              </div>
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  );
}
