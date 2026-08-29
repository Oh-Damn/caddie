import type { RunningApp } from '@companion/protocol';
import { Bookmark, ChevronLeft, ChevronRight } from 'lucide-react';
import type { ReactNode } from 'react';
import { AppIcon } from './AppIcon';

type Props = {
  appName: string;
  bundleId: string;
  windowTitle: string;
  recentApps: RunningApp[];
  unread?: number | null;
  pinned?: boolean;
  showHistory?: boolean;
  canBack?: boolean;
  canForward?: boolean;
  onFocus: () => void;
  onFocusApp: (bundleId: string) => void;
  onHistory?: (direction: 'back' | 'forward') => void;
  onTogglePin?: () => void;
};

export function ContextCard({
  appName,
  bundleId,
  windowTitle,
  recentApps,
  unread = null,
  pinned = false,
  showHistory = false,
  canBack = true,
  canForward = true,
  onFocus,
  onFocusApp,
  onHistory,
  onTogglePin,
}: Props) {
  const location = windowTitle.trim();
  const history = showHistory && onHistory;

  return (
    <div className="flex w-full shrink-0 items-center gap-1.5">
      <div className="readout flex min-w-0 flex-1 items-center">
        <button
          type="button"
          className="flex min-w-0 flex-1 items-center gap-2.5 px-2.5 py-2 text-left"
          onClick={onFocus}
        >
          <AppIcon
            bundleId={bundleId}
            name={appName}
            className="h-8 w-8 shrink-0 rounded-app object-cover"
          />
          <div className="relative z-[1] min-w-0 flex-1">
            <p className="truncate font-mono text-sm tracking-wide text-lcd">{appName}</p>
            {location ? <p className="type-meta mt-0.5 truncate">{location}</p> : null}
          </div>
          {unread != null && unread > 0 ? (
            <span
              className="relative z-[1] shrink-0 font-mono text-sm text-lcd"
              aria-label="Unread"
            >
              !
            </span>
          ) : null}
        </button>
        {history ? (
          <div className="relative z-[1] flex shrink-0 items-center self-stretch pr-1">
            <span className="mr-0.5 h-5 w-px bg-app-border" aria-hidden />
            <HistoryKey
              label="Back"
              disabled={!canBack}
              onClick={() => onHistory?.('back')}
            >
              <ChevronLeft className="h-4 w-4" strokeWidth={2.2} />
            </HistoryKey>
            <HistoryKey
              label="Forward"
              disabled={!canForward}
              onClick={() => onHistory?.('forward')}
            >
              <ChevronRight className="h-4 w-4" strokeWidth={2.2} />
            </HistoryKey>
          </div>
        ) : null}
      </div>
      {onTogglePin ? (
        <button
          type="button"
          aria-label={
            pinned
              ? `Remove bookmark ${location || appName}`
              : `Bookmark ${location || appName}`
          }
          className="key flex h-8 w-8 shrink-0 items-center justify-center"
          onClick={onTogglePin}
        >
          <Bookmark
            className={`h-3.5 w-3.5 ${pinned ? 'fill-accent text-accent' : 'text-app-muted'}`}
          />
        </button>
      ) : null}
      {recentApps.length > 0 ? (
        <div className="flex shrink-0 items-center gap-1">
          {recentApps.map((app) => (
            <button
              key={app.bundleId}
              type="button"
              className="key h-8 w-8 shrink-0 overflow-hidden"
              aria-label={`Switch to ${app.name}`}
              onClick={() => onFocusApp(app.bundleId)}
            >
              <AppIcon
                bundleId={app.bundleId}
                name={app.name}
                className="h-full w-full object-cover"
              />
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function HistoryKey({
  label,
  onClick,
  disabled = false,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      className="flex h-full min-h-8 w-8 items-center justify-center text-lcd disabled:opacity-30"
      onClick={onClick}
    >
      {children}
    </button>
  );
}
