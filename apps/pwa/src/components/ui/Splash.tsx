import { useEffect, useState } from 'react';
import { PoweredBy } from './PoweredBy';

const HOLD_MS = 1400;
const FADE_MS = 320;

export function Splash() {
  const [leaving, setLeaving] = useState(false);
  const [gone, setGone] = useState(false);

  useEffect(() => {
    const out = window.setTimeout(() => setLeaving(true), HOLD_MS);
    const done = window.setTimeout(() => setGone(true), HOLD_MS + FADE_MS);
    return () => {
      window.clearTimeout(out);
      window.clearTimeout(done);
    };
  }, []);

  if (gone) return null;

  return (
    <div
      aria-hidden
      className="pointer-events-none fixed inset-0 z-50 flex flex-col items-center justify-center bg-app"
      style={{
        opacity: leaving ? 0 : 1,
        transition: `opacity ${FADE_MS}ms ease-out`,
      }}
    >
      <PoweredBy variant="splash" />
    </div>
  );
}
