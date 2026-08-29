import type { CSSProperties, ReactNode } from 'react';
import { AgentAlertStrip } from '../components/ai/AgentAlertStrip';
import { AliveBackground } from '../components/home/AliveBackground';
import { ApprovalStrip } from '../components/home/ApprovalStrip';
import { ErrorPopup } from '../components/ui/ErrorPopup';
import { InstallPopup } from '../components/ui/InstallPopup';
import { mediaNowPlaying } from '../companion/nowPlaying';
import { useCompanion } from '../companion/useCompanion';
import { DEFAULT_ACCENT } from '../lib/accent';
import { usePreferences } from '../lib/usePreferences';
import { ROUTES } from '../routes';
import { useRoute } from '../useRoute';

type Props = {
  children: ReactNode;
};

export function AppShell({ children }: Props) {
  const { error, setError, appState, layout, command } = useCompanion();
  const { prefs } = usePreferences();
  const { route } = useRoute();
  const np = mediaNowPlaying(appState, appState?.pluginId || layout?.screen);
  const onHome = route === ROUTES.home;
  const approval = route !== ROUTES.connect ? (appState?.approval ?? null) : null;
  const agents = appState?.agents ?? null;

  const agentAlert =
    agents &&
    agents.waiting > 0 &&
    route !== ROUTES.connect &&
    route !== ROUTES.ai &&
    !approval
      ? agents
      : null;
  const accentStyle = {
    '--accent': DEFAULT_ACCENT.accent,
    '--accent-text': DEFAULT_ACCENT.accentText,
  } as CSSProperties;

  return (
    <div className="relative h-svh max-h-svh overflow-hidden" style={accentStyle}>
      <AliveBackground
        active={onHome && prefs.aliveBackground}
        playing={Boolean(np?.playing)}
        artworkUrl={np?.artworkUrl}
      />

      <div className="relative z-10 flex h-svh max-h-svh flex-col overflow-hidden bg-transparent pl-[max(1rem,env(safe-area-inset-left))] pr-[max(1rem,env(safe-area-inset-right))] pt-[max(0.75rem,env(safe-area-inset-top))] pb-[max(0.75rem,env(safe-area-inset-bottom))] text-app-text">
        {approval ? (
          <div className="mx-auto mb-1.5 w-full max-w-md shrink-0 landscape:max-w-4xl">
            <ApprovalStrip approval={approval} onCommand={command} />
          </div>
        ) : null}
        {agentAlert ? (
          <div className="mx-auto mb-1.5 w-full max-w-md shrink-0 landscape:max-w-4xl">
            <AgentAlertStrip summary={agentAlert} />
          </div>
        ) : null}
        {children}
        <InstallPopup />
        {error ? <ErrorPopup message={error} onDismiss={() => setError(null)} /> : null}
      </div>
    </div>
  );
}
