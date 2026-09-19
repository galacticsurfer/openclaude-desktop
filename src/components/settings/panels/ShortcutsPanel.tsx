import { SHORTCUTS } from '@/hooks/useHotkeys';

export function ShortcutsPanel() {
  const groups = SHORTCUTS.reduce<Record<string, typeof SHORTCUTS>>((acc, s) => {
    (acc[s.group] ??= []).push(s);
    return acc;
  }, {});

  return (
    <div className="space-y-6">
      {Object.entries(groups).map(([group, items]) => (
        <section key={group}>
          <h4 className="mb-2 text-[13px] font-semibold text-ink">{group}</h4>
          <dl className="divide-y divide-line">
            {items.map((s) => (
              <div key={s.keys} className="flex items-center justify-between py-2">
                <dt className="text-[13.5px] text-ink-soft">{s.label}</dt>
                <dd className="flex gap-1">
                  {s.keys.split('+').map((k) => (
                    <kbd
                      key={k}
                      className="rounded border border-line-strong bg-sunken px-1.5 py-0.5 font-mono text-[11.5px] text-ink-soft"
                    >
                      {k}
                    </kbd>
                  ))}
                </dd>
              </div>
            ))}
          </dl>
        </section>
      ))}
      <p className="text-[12.5px] text-ink-faint">
        Remappable shortcuts are planned for a future release.
      </p>
    </div>
  );
}
