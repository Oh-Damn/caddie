import type { AgentsSummary } from '@companion/protocol';
import { Sparkles } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { haptic } from '../../companion/session';
import { chimeSound } from '../../lib/sound';
import { navigate, ROUTES } from '../../routes';
import { IconButton } from '../ui/IconButton';

type Props = {
  summary: AgentsSummary | null;
};

const FLASH_MS = 4000;

export function AgentsNavButton({ summary }: Props) {
  const waiting = summary?.waiting ?? 0;
  const wasWaiting = useRef(0);
  const [flash, setFlash] = useState(false);

  useEffect(() => {
    if (waiting > wasWaiting.current) {
      haptic('warning');
      chimeSound();
      setFlash(true);
    }
    if (waiting === 0) setFlash(false);
    wasWaiting.current = waiting;
  }, [waiting]);

  useEffect(() => {
    if (!flash) return;
    const id = window.setTimeout(() => setFlash(false), FLASH_MS);
    return () => window.clearTimeout(id);
  }, [flash]);

  const label = waiting > 0 ? `Agents, ${waiting} waiting` : 'Agents';

  return (
    <IconButton
      label={label}
      variant="chip"
      size="sm"
      accent={flash}
      onClick={() => navigate(ROUTES.ai)}
    >
      <Sparkles className={`h-4 w-4 ${flash ? 'agent-pulse' : ''}`} strokeWidth={2} />
    </IconButton>
  );
}
