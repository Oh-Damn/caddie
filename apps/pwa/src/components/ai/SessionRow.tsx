import type { AgentSession } from '@companion/protocol';
import { CircleAlert, GitBranch } from 'lucide-react';
import { agoLabel, formatTokens } from '../../companion/agents';

type Props = {
  session: AgentSession;
  provider?: string;
  onFocus?: () => void;
};

export function SessionRow({ session, provider, onFocus }: Props) {
  const body = (
    <>
      <span
        className="h-1.5 w-1.5 shrink-0 rounded-full"
        style={{
          background: session.waiting
            ? 'var(--status-wait)'
            : session.active
              ? 'var(--status-ready)'
              : 'var(--status-idle)',
        }}
        aria-hidden
      />
      <div className="min-w-0 flex-1">
        <span className="type-secondary block truncate">
          {session.project || 'Untitled'}
        </span>
        <span className="type-meta mt-0.5 flex min-w-0 items-center gap-1.5">
          {provider ? (
            <>
              <span className="truncate">{provider}</span>
              <span aria-hidden>·</span>
            </>
          ) : null}
          <span className="truncate">{session.model || 'unknown'}</span>
          {session.branch ? (
            <>
              <GitBranch className="h-2.5 w-2.5 shrink-0" strokeWidth={2.4} />
              <span className="truncate">{session.branch}</span>
            </>
          ) : null}
        </span>
      </div>
      {session.waiting ? (
        <CircleAlert
          className="h-4 w-4 shrink-0"
          style={{ color: 'var(--status-wait)' }}
          strokeWidth={2.2}
        />
      ) : null}
      <div className="shrink-0 text-right">
        {session.tokens > 0 ? (
          <span className="type-stat block !text-[0.8rem]">
            {formatTokens(session.tokens)}
          </span>
        ) : session.detail ? (
          <span className="type-meta block">{session.detail}</span>
        ) : null}
        <span className="type-meta mt-0.5 block">{agoLabel(session.lastActive)}</span>
      </div>
    </>
  );

  const className =
    'flex min-h-touch w-full items-center gap-3 border-b border-app-border px-3 py-2.5 last:border-b-0';

  if (onFocus) {
    return (
      <button
        type="button"
        className={`${className} text-left`}
        aria-label={`Open ${session.project || 'agent'} on the Mac`}
        onClick={onFocus}
      >
        {body}
      </button>
    );
  }

  return <div className={className}>{body}</div>;
}
