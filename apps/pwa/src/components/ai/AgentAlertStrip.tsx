import type { AgentsSummary } from '@companion/protocol';
import { ChevronRight, CircleAlert } from 'lucide-react';
import { navigate, ROUTES } from '../../routes';

type Props = {
  summary: AgentsSummary;
};

export function AgentAlertStrip({ summary }: Props) {
  const label =
    summary.waiting === 1 ? 'An agent needs you' : `${summary.waiting} agents need you`;

  return (
    <button
      type="button"
      className="faceplate flex w-full items-center gap-2 px-2.5 py-2 text-left"
      onClick={() => navigate(ROUTES.ai)}
    >
      <CircleAlert
        className="h-4 w-4 shrink-0"
        style={{ color: 'var(--status-wait)' }}
        strokeWidth={2.2}
      />
      <span className="min-w-0 flex-1">
        <span className="type-secondary block truncate">{label}</span>
        <span className="type-meta mt-0.5 block truncate">Waiting on a prompt</span>
      </span>
      <ChevronRight className="h-4 w-4 shrink-0 text-app-muted" strokeWidth={2.2} />
    </button>
  );
}
