import { useSettingsStore } from '@/stores/useSettingsStore';
import { Switch } from '@/components/ui/Switch';
import { Select } from '@/components/ui/Select';
import { Group, Row } from '../SettingsDialog';
import type { SendKeyPreference } from '@/types';

export function GeneralPanel() {
  const { settings, set } = useSettingsStore();
  if (!settings) return null;

  return (
    <>
      <Group title="Startup">
        <Switch
          checked={settings['general.restoreLastConversation']}
          onChange={(v) => void set('general.restoreLastConversation', v)}
          label="Reopen the last conversation on launch"
          description="Restores the conversation you were in, along with the window size and position."
        />
        <Switch
          checked={settings['general.trayEnabled']}
          onChange={(v) => void set('general.trayEnabled', v)}
          label="Keep running in the system tray"
          description="Closing the window hides it instead of quitting, and a tray icon opens it again. Takes effect after a restart. Some desktops have nowhere to put a tray icon — if none appears, the window closes normally."
        />
      </Group>

      <Group title="Composer">
        <Row
          label="Send with"
          description="Choose which key sends a message. The other inserts a new line."
          control={
            <Select
              value={settings['general.sendKey']}
              onChange={(e) => void set('general.sendKey', e.target.value as SendKeyPreference)}
              aria-label="Send with"
            >
              <option value="enter">Enter</option>
              <option value="ctrlEnter">Ctrl + Enter</option>
            </Select>
          }
        />
      </Group>
    </>
  );
}
