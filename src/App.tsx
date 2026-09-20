import { useEffect, useState } from 'react';
import { AppShell } from '@/components/layout/AppShell';
import { Welcome } from '@/components/onboarding/Welcome';
import { Toasts } from '@/components/ui/Toasts';
import { useSettingsStore } from '@/stores/useSettingsStore';
import { useConversationStore } from '@/stores/useConversationStore';
import { useTheme } from '@/hooks/useTheme';
import { useStreamEvents } from '@/hooks/useStreamEvents';
import { Spinner } from '@/components/ui/Spinner';
import { AppError } from '@/services/ipc';

export default function App() {
  const { settings, claudeCode, loading, load, refreshModels } = useSettingsStore();
  const { loadConversations, loadProjects, open } = useConversationStore();
  const [fatal, setFatal] = useState<string | null>(null);
  const [skippedOnboarding, setSkippedOnboarding] = useState(false);

  useTheme();
  useStreamEvents();

  useEffect(() => {
    void (async () => {
      try {
        await load();
      } catch (err) {
        setFatal(AppError.from(err).message);
        return;
      }

      // These are best-effort: the app is usable offline and without a key.
      void refreshModels();
      await loadProjects();
      await loadConversations();

      const s = useSettingsStore.getState().settings;
      const last = s?.['ui.lastConversationId'];
      if (s?.['general.restoreLastConversation'] && last) {
        // The conversation may have been deleted since; failing to reopen it
        // must not block startup.
        try {
          await open(last);
        } catch {
          /* fall through to the empty state */
        }
      }
    })();
  }, [load, refreshModels, loadConversations, loadProjects, open]);

  if (fatal) {
    return (
      <div className="flex h-full items-center justify-center bg-canvas p-8">
        <div className="max-w-md text-center">
          <h1 className="mb-2 text-[16px] font-semibold text-ink">
            OpenClaude could not start
          </h1>
          <p className="text-[13.5px] leading-relaxed text-ink-soft">{fatal}</p>
          <p className="mt-4 text-[12.5px] text-ink-faint">
            If this persists, check the log directory listed in Settings → Advanced, or file an
            issue on GitHub.
          </p>
        </div>
      </div>
    );
  }

  if (loading || !settings) {
    return (
      <div className="flex h-full items-center justify-center bg-canvas">
        <Spinner label="Starting OpenClaude" className="size-6" />
      </div>
    );
  }

  const needsOnboarding =
    !settings['ui.onboarded'] && !claudeCode?.installed && !skippedOnboarding;

  if (needsOnboarding) {
    return (
      <>
        <Welcome onDone={() => setSkippedOnboarding(true)} />
        <Toasts />
      </>
    );
  }

  return <AppShell />;
}
