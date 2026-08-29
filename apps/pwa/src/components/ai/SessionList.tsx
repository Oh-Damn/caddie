import type { AgentProvider, AgentSession } from '@companion/protocol';
import { SessionRow } from './SessionRow';

const SESSION_CAP = 5;

type Row = {
  key: string;
  session: AgentSession;
  providerId: string;
  providerName: string;
  available: boolean;
};

type Props = {
  providers: AgentProvider[];
  focusFor?: Record<string, () => void>;
};

export function SessionList({ providers, focusFor }: Props) {
  const rows: Row[] = providers
    .flatMap((provider) =>
      provider.sessions.map((session) => ({
        key: `${provider.id}:${session.id}`,
        session,
        providerId: provider.id,
        providerName: provider.name,
        available: provider.available,
      })),
    )
    .sort((a, b) => {
      if (a.session.waiting !== b.session.waiting) return a.session.waiting ? -1 : 1;
      return Date.parse(b.session.lastActive) - Date.parse(a.session.lastActive);
    })
    .slice(0, SESSION_CAP);

  if (rows.length === 0) {
    return (
      <section className="faceplate px-3 py-4">
        <p className="type-meta">No recent agents</p>
      </section>
    );
  }

  return (
    <section className="faceplate">
      {rows.map((row) => (
        <SessionRow
          key={row.key}
          session={row.session}
          provider={row.providerName}
          onFocus={
            row.available && focusFor?.[row.providerId]
              ? focusFor[row.providerId]
              : undefined
          }
        />
      ))}
    </section>
  );
}
