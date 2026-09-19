import { useEffect, useState } from 'react';
import { FolderOpen, Trash2 } from 'lucide-react';
import * as api from '@/services/api';
import { AppError } from '@/services/ipc';
import { useConversationStore } from '@/stores/useConversationStore';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useUIStore } from '@/stores/useUIStore';
import { Dialog } from '@/components/ui/Dialog';
import { Button } from '@/components/ui/Button';
import { Field, Input, Textarea } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';

const COLORS = ['#B4522F', '#8A6D3B', '#3F6B52', '#3C5F82', '#6B4A7A', '#7A4A4A'];

export function ProjectDialog({ projectId }: { projectId?: string }) {
  const { closeOverlay, toast, confirm } = useUIStore();
  const { loadProjects, loadConversations } = useConversationStore();
  const models = useSettingsStore((s) => s.models);

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [instructions, setInstructions] = useState('');
  const [workingDir, setWorkingDir] = useState<string | null>(null);
  const [defaultModel, setDefaultModel] = useState<string | null>(null);
  const [color, setColor] = useState<string | null>(COLORS[0] ?? null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!projectId) return;
    void api
      .getProject(projectId)
      .then((p) => {
        setName(p.name);
        setDescription(p.description);
        setInstructions(p.instructions);
        setWorkingDir(p.workingDir);
        setDefaultModel(p.defaultModel);
        setColor(p.color);
      })
      .catch((e) => toast('error', AppError.from(e).message));
  }, [projectId, toast]);

  async function save() {
    setSaving(true);
    try {
      const input = {
        name: name.trim(),
        description: description.trim(),
        instructions: instructions.trim(),
        workingDir,
        defaultModel,
        color,
      };
      if (projectId) await api.updateProject(projectId, input);
      else await api.createProject(input);
      await loadProjects();
      await loadConversations();
      closeOverlay();
    } catch (err) {
      toast('error', AppError.from(err).message);
    } finally {
      setSaving(false);
    }
  }

  async function pickFolder() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const dir = await open({ directory: true, multiple: false });
      if (typeof dir === 'string') setWorkingDir(dir);
    } catch {
      toast('error', 'Could not open the folder picker.');
    }
  }

  return (
    <Dialog
      open
      onClose={closeOverlay}
      title={projectId ? 'Edit project' : 'New project'}
      description="Projects group related conversations and can carry standing instructions."
      size="lg"
      footer={
        <>
          {projectId && (
            <Button
              variant="ghost"
              className="mr-auto text-danger"
              onClick={() =>
                confirm({
                  title: 'Delete this project?',
                  body: 'Its conversations are kept — they simply become ungrouped.',
                  confirmLabel: 'Delete project',
                  destructive: true,
                  onConfirm: async () => {
                    await api.deleteProject(projectId);
                    await loadProjects();
                    await loadConversations();
                    useUIStore.getState().setActiveProject(null);
                    closeOverlay();
                  },
                })
              }
            >
              <Trash2 size={14} /> Delete
            </Button>
          )}
          <Button variant="ghost" onClick={closeOverlay}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => void save()} disabled={!name.trim()} loading={saving}>
            {projectId ? 'Save changes' : 'Create project'}
          </Button>
        </>
      }
    >
      <div className="space-y-4 p-5">
        <Field label="Name" htmlFor="p-name">
          <Input
            id="p-name"
            data-autofocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Backend API"
          />
        </Field>

        <Field label="Description" htmlFor="p-desc" hint="Optional. Only for your own reference.">
          <Input
            id="p-desc"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="FastAPI service backed by MongoDB"
          />
        </Field>

        <Field
          label="Project instructions"
          htmlFor="p-inst"
          hint="Prepended to the system prompt of every conversation in this project. The chat header shows when they are active."
        >
          <Textarea
            id="p-inst"
            rows={6}
            value={instructions}
            onChange={(e) => setInstructions(e.target.value)}
            placeholder={
              'This is a FastAPI backend using MongoDB.\nFollow the existing architecture.\nPrefer async APIs and Python 3.12.\nExplain any new dependency before introducing it.'
            }
          />
        </Field>

        <Field
          label="Working folder"
          hint="Recorded for reference and used as the starting point of the file picker. Nothing in it is read or uploaded automatically."
        >
          <div className="flex gap-2">
            <Input
              value={workingDir ?? ''}
              onChange={(e) => setWorkingDir(e.target.value || null)}
              placeholder="~/code/backend-api"
              className="font-mono text-[13px]"
            />
            <Button variant="secondary" onClick={() => void pickFolder()}>
              <FolderOpen size={14} /> Browse
            </Button>
          </div>
        </Field>

        <div className="grid grid-cols-2 gap-4">
          <Field label="Default model" hint="Used for new conversations in this project.">
            <Select
              value={defaultModel ?? ''}
              onChange={(e) => setDefaultModel(e.target.value || null)}
            >
              <option value="">Use the app default</option>
              {models.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.displayName}
                </option>
              ))}
            </Select>
          </Field>

          <Field label="Colour">
            <div className="flex gap-1.5 pt-1">
              {COLORS.map((c) => (
                <button
                  key={c}
                  type="button"
                  aria-label={`Colour ${c}`}
                  aria-pressed={color === c}
                  onClick={() => setColor(c)}
                  style={{ backgroundColor: c }}
                  className={`size-6 rounded-full transition-transform ${
                    color === c ? 'ring-2 ring-accent ring-offset-2 ring-offset-[rgb(var(--c-raised))]' : 'hover:scale-110'
                  }`}
                />
              ))}
            </div>
          </Field>
        </div>
      </div>
    </Dialog>
  );
}
