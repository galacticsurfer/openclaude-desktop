import { useSettingsStore } from '@/stores/useSettingsStore';
import { Switch } from '@/components/ui/Switch';
import { Select } from '@/components/ui/Select';
import { Button } from '@/components/ui/Button';
import { useUIStore } from '@/stores/useUIStore';
import { Group, Row } from '../SettingsDialog';

export function NotificationsPanel() {
  const { settings, set } = useSettingsStore();
  const toast = useUIStore((s) => s.toast);
  if (!settings) return null;

  async function sendTest() {
    try {
      const { isPermissionGranted, requestPermission, sendNotification } = await import(
        '@tauri-apps/plugin-notification'
      );
      let granted = await isPermissionGranted();
      if (!granted) granted = (await requestPermission()) === 'granted';
      if (!granted) {
        toast('error', 'Your desktop denied notification permission.');
        return;
      }
      await sendNotification({
        title: 'OpenClaude Desktop',
        body: 'Notifications are working.',
      });
    } catch {
      toast('error', 'Could not send a notification.');
    }
  }

  return (
    <Group
      title="Desktop notifications"
      description="Shown through your desktop's own notification service, only while the OpenClaude window is not focused."
    >
      <Switch
        checked={settings['notifications.enabled']}
        onChange={(v) => void set('notifications.enabled', v)}
        label="Notify when Claude finishes responding"
        description="Only for replies that took a while — a quick answer you are watching does not need one."
      />
      <Row
        label="Minimum response time"
        description="Replies faster than this never raise a notification."
        control={
          <Select
            value={String(settings['notifications.minDurationMs'])}
            onChange={(e) => void set('notifications.minDurationMs', Number(e.target.value))}
            disabled={!settings['notifications.enabled']}
            aria-label="Minimum response time before notifying"
          >
            <option value="0">Always</option>
            <option value="3000">3 seconds</option>
            <option value="8000">8 seconds</option>
            <option value="20000">20 seconds</option>
            <option value="60000">1 minute</option>
          </Select>
        }
      />
      <Row
        label="Test"
        description="Send a notification now to check your desktop is configured."
        control={
          <Button variant="secondary" size="sm" onClick={() => void sendTest()}>
            Send test notification
          </Button>
        }
      />
    </Group>
  );
}
