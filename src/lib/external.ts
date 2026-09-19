import { useUIStore } from '@/stores/useUIStore';

/**
 * Hand a URL to the system browser.
 *
 * Only http(s) and mailto are allowed: model output is untrusted, and a
 * `file://` or custom-scheme link should never be launched on the user's
 * behalf from a chat transcript.
 */
export async function openExternal(url: string): Promise<void> {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    useUIStore.getState().toast('error', 'That link is not a valid URL.');
    return;
  }

  if (!['http:', 'https:', 'mailto:'].includes(parsed.protocol)) {
    useUIStore
      .getState()
      .toast('error', `Links of type "${parsed.protocol}" are not opened automatically.`);
    return;
  }

  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(parsed.toString());
  } catch {
    useUIStore.getState().toast('error', 'Could not open that link.');
  }
}
