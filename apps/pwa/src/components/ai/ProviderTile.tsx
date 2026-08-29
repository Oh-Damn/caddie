import type { AgentProvider } from '@companion/protocol';
import {
  formatTokens,
  resetLabel,
  TONE_VAR,
  toneFor,
  type Tone,
} from '../../companion/agents';
import { UsageRing } from './UsageRing';

type CursorKind = 'wait' | 'unread' | 'idle';

const CURSOR_RING: Record<CursorKind, { percent: number; tone: Tone; label: string }> = {
  wait: { percent: 100, tone: 'wait', label: 'WAIT' },
  unread: { percent: 50, tone: 'ready', label: 'UNREAD' },
  idle: { percent: 0, tone: 'idle', label: 'IDLE' },
};

type Props = {
  provider: AgentProvider;
  onFocus?: () => void;
};

function countLine(n: number, word: string): string {
  return n === 1 ? `1 ${word}` : `${n} ${word}`;
}

function claudeLine(provider: AgentProvider): string {
  if (provider.waiting > 0) return countLine(provider.waiting, 'waiting');
  if (provider.active > 0) return countLine(provider.active, 'active');
  return 'Idle';
}

function unreadCount(provider: AgentProvider): number {
  return provider.sessions.filter(
    (session) =>
      !session.waiting &&
      (session.active || session.detail.toLowerCase().includes('unread')),
  ).length;
}

function cursorKind(provider: AgentProvider): CursorKind {
  if (provider.waiting > 0) return 'wait';
  if (unreadCount(provider) > 0) return 'unread';
  return 'idle';
}

function cursorLine(provider: AgentProvider, kind: CursorKind): string {
  if (kind === 'wait') return countLine(provider.waiting, 'waiting');
  if (kind === 'unread') {
    const n = unreadCount(provider);
    return n > 1 ? countLine(n, 'unread') : 'unread';
  }
  return 'idle';
}

export function ProviderTile({ provider, onFocus }: Props) {
  const session = provider.session;
  const kind = cursorKind(provider);
  const ring = session
    ? {
        percent: session.percent,
        tone: toneFor(session.percent),
        label: `${session.percent}%`,
        center: `${session.percent}%`,
        stat: true,
      }
    : {
        ...CURSOR_RING[kind],
        center: CURSOR_RING[kind].label,
        stat: false,
      };

  const status =
    provider.blocked ?? (session ? claudeLine(provider) : cursorLine(provider, kind));
  const center = provider.blocked && !session ? '--' : ring.center;

  const body = (
    <>
      <header className="w-full min-w-0">
        <p className="type-secondary truncate">{provider.name}</p>
        <p className="type-meta mt-0.5 truncate">{status}</p>
      </header>
      <UsageRing percent={ring.percent} tone={ring.tone} label={ring.label}>
        <span
          className={ring.stat ? 'type-stat' : 'type-micro'}
          style={ring.stat ? undefined : { color: TONE_VAR[ring.tone] }}
        >
          {center}
        </span>
      </UsageRing>
      {session ? (
        <div className="flex w-full min-w-0 flex-col items-center gap-0.5">
          {session.limit === null ? null : (
            <p className="type-meta">
              {formatTokens(session.used)} / {formatTokens(session.limit)}
            </p>
          )}
          <p className="type-meta truncate">
            {provider.estimated ? `${resetLabel(session)} · est.` : resetLabel(session)}
          </p>
        </div>
      ) : null}
    </>
  );

  const className =
    'faceplate flex h-full min-h-touch w-full flex-col items-center gap-2 p-3';

  if (onFocus) {
    return (
      <button
        type="button"
        className={`${className} text-center`}
        aria-label={`Open ${provider.name} on the Mac`}
        onClick={onFocus}
      >
        {body}
      </button>
    );
  }

  return <section className={className}>{body}</section>;
}
