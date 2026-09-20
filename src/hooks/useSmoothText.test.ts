import { describe, expect, it } from 'vitest';
import { revealStep } from './useSmoothText';

describe('revealStep', () => {
  it('reveals nothing when the display has caught up', () => {
    expect(revealStep(0, 16)).toBe(0);
    expect(revealStep(-5, 16)).toBe(0);
  });

  it('never outruns the text actually received', () => {
    expect(revealStep(3, 1000)).toBe(3);
  });

  it('always advances at least one character, however small the backlog', () => {
    expect(revealStep(1, 1)).toBe(1);
  });

  it('keeps pace with the CLI cadence, revealing on nearly every frame', () => {
    // Measured from the real CLI: ~25 characters every ~60ms.
    const FRAME = 16;
    let received = 0;
    let shown = 0;
    let idle = 0;

    for (let t = 0; t < 3000; t += FRAME) {
      if (t % 60 < FRAME) received += 25;
      const step = revealStep(received - shown, FRAME);
      if (step === 0) idle++;
      shown += step;
    }

    // Without smoothing the text would move on 16 frames a second out of 60.
    const frames = Math.ceil(3000 / FRAME);
    expect(idle / frames).toBeLessThan(0.15);
    // And it stays close behind rather than drifting further and further.
    expect(received - shown).toBeLessThan(50);
  });

  it('goes faster when more text is waiting', () => {
    expect(revealStep(1000, 16)).toBeGreaterThan(revealStep(100, 16));
  });
});
