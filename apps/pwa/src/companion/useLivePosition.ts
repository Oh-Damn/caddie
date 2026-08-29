import { useEffect, useState } from 'react';

const STALE_MS = 20000;

export function useLivePosition(
  positionSec: number,
  durationSec: number,
  playing: boolean,
  connected: boolean,
): number {
  const [anchor, setAnchor] = useState(() => ({ at: Date.now(), pos: positionSec }));
  const [, tick] = useState(0);

  useEffect(() => {
    setAnchor({ at: Date.now(), pos: positionSec });
  }, [positionSec]);

  const running = playing && connected;

  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => tick((n) => n + 1), 1000);
    return () => window.clearInterval(id);
  }, [running]);

  if (!running) return positionSec;

  const drift = Math.min(Date.now() - anchor.at, STALE_MS);
  const live = anchor.pos + drift / 1000;
  return durationSec > 0 ? Math.min(live, durationSec) : live;
}
