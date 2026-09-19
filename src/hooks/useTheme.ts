import { useEffect } from 'react';
import { useSettingsStore } from '@/stores/useSettingsStore';

/**
 * Apply the theme, font size, density and motion preferences to <html>.
 *
 * "system" follows the desktop: WebKitGTK maps the GNOME/KDE colour scheme
 * onto `prefers-color-scheme`, so no extra platform call is needed.
 */
export function useTheme(): void {
  const theme = useSettingsStore((s) => s.settings?.['appearance.theme'] ?? 'system');
  const fontSize = useSettingsStore((s) => s.settings?.['appearance.fontSize'] ?? 15);
  const compact = useSettingsStore((s) => s.settings?.['appearance.compact'] ?? false);
  const reducedMotion = useSettingsStore((s) => s.settings?.['appearance.reducedMotion'] ?? false);

  useEffect(() => {
    const root = document.documentElement;
    const media = window.matchMedia('(prefers-color-scheme: dark)');

    const apply = () => {
      const dark = theme === 'dark' || (theme === 'system' && media.matches);
      root.classList.toggle('dark', dark);
      root.style.colorScheme = dark ? 'dark' : 'light';
    };

    apply();
    if (theme !== 'system') return;
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, [theme]);

  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty('--fs-body', `${Math.min(Math.max(fontSize, 12), 20)}px`);
    root.classList.toggle('density-compact', compact);
    root.classList.toggle('reduce-motion', reducedMotion);
  }, [fontSize, compact, reducedMotion]);
}
