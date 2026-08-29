import type { AgentProvider } from '@companion/protocol';
import { useEffect, useState } from 'react';
import { ProviderTile } from '../components/ai/ProviderTile';
import { SessionList } from '../components/ai/SessionList';
import { Skeleton } from '../components/ui/Skeleton';
import { useCommand } from '../companion/useCommand';
import { useCompanion } from '../companion/useCompanion';
import { StackLayout } from '../layouts/StackLayout';
import { navigate, ROUTES } from '../routes';

const BUNDLE_FOR: Record<string, string> = {
  cursor: 'com.todesktop.230313mzl4w4u92',
  'claude-desktop': 'com.anthropic.claudefordesktop',
};

function isVisible(provider: AgentProvider): boolean {
  if (provider.sessions.length > 0 || provider.session || provider.week) return true;

  return provider.available;
}

export function AiScreen() {
  const { agents } = useCompanion();
  const go = useCommand();
  const [, setTick] = useState(0);

  useEffect(() => {
    const id = window.setInterval(() => setTick((n) => n + 1), 30_000);
    return () => window.clearInterval(id);
  }, []);

  const visible = agents?.providers.filter(isVisible) ?? [];
  const focusFor: Record<string, () => void> = Object.fromEntries(
    Object.entries(BUNDLE_FOR).map(([id, bundle]) => [
      id,
      () => go('app.focus', null, bundle),
    ]),
  );

  return (
    <StackLayout title="Agents" onBack={() => navigate(ROUTES.home)}>
      <div className="flex flex-col gap-4 pb-4">
        {!agents ? (
          <>
            <div className="grid grid-cols-2 gap-2">
              <Skeleton className="h-44 w-full" />
              <Skeleton className="h-44 w-full" />
            </div>
            <Skeleton className="h-28 w-full" />
          </>
        ) : (
          <>
            {visible.length > 0 ? (
              <div className="grid grid-cols-2 gap-2">
                {visible.map((provider) => (
                  <ProviderTile
                    key={provider.id}
                    provider={provider}
                    onFocus={provider.available ? focusFor[provider.id] : undefined}
                  />
                ))}
              </div>
            ) : null}

            <SessionList providers={visible} focusFor={focusFor} />
          </>
        )}
      </div>
    </StackLayout>
  );
}
