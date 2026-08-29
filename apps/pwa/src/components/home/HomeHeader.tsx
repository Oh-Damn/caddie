import { ClipboardList, LayoutGrid, Settings } from 'lucide-react';
import type { ReactNode } from 'react';
import { useCompanion } from '../../companion/useCompanion';
import { navigate, ROUTES } from '../../routes';
import { AgentsNavButton } from '../ai/AgentsNavButton';
import { BrandMark } from '../ui/BrandMark';
import { STATUS_LABEL, STATUS_TONE } from '../ui/ConnectionDot';
import { IconButton } from '../ui/IconButton';

type Props = {
  extra?: ReactNode;
};

export function HomeHeader({ extra }: Props) {
  const { status, appState } = useCompanion();

  return (
    <header className="flex shrink-0 items-center gap-2">
      <div className="flex min-w-0 shrink-0 items-center gap-2">
        <BrandMark className="h-8 w-8" />
        <span className="type-micro" style={{ color: STATUS_TONE[status] }}>
          {STATUS_LABEL[status]}
        </span>
      </div>
      <div className="min-w-0 flex-1 self-center">
        {extra ? (
          <div className="hidden h-full min-w-0 landscape:flex">{extra}</div>
        ) : null}
      </div>
      <nav className="flex shrink-0 items-center gap-1">
        <AgentsNavButton summary={appState?.agents ?? null} />
        <IconButton
          label="Open clipboard"
          variant="chip"
          size="sm"
          onClick={() => navigate(ROUTES.clipboard)}
        >
          <ClipboardList className="h-4 w-4" strokeWidth={2} />
        </IconButton>
        <IconButton
          label="Open apps"
          variant="chip"
          size="sm"
          onClick={() => navigate(ROUTES.apps)}
        >
          <LayoutGrid className="h-4 w-4" strokeWidth={2} />
        </IconButton>
        <IconButton
          label="Open settings"
          variant="chip"
          size="sm"
          onClick={() => navigate(ROUTES.settings)}
        >
          <Settings className="h-4 w-4" strokeWidth={2} />
        </IconButton>
      </nav>
    </header>
  );
}
