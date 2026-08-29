import type { BrowserTab } from '@companion/protocol';
import { Pause, Play } from 'lucide-react';
import { parseTabTitle } from '../../lib/tabTitle';

type Props = {
  tab: BrowserTab;
  playing?: boolean;
  onActivate: (tab: BrowserTab) => void;
  onToggleMedia?: (tab: BrowserTab) => void;
};

export function TabRow({ tab, playing = false, onActivate, onToggleMedia }: Props) {
  const { page, site, unread } = parseTabTitle(tab.title);
  const showToggle = Boolean((tab.media || tab.audible) && onToggleMedia);

  return (
    <div
      className={`flex min-h-touch w-full min-w-0 items-center ${
        tab.active ? 'bg-app-well' : ''
      }`}
    >
      <button
        type="button"
        aria-pressed={tab.active}
        aria-label={playing ? `${tab.title}, playing` : tab.title}
        className="flex min-h-touch min-w-0 flex-1 items-center gap-2.5 px-3 py-2 text-left"
        onClick={() => onActivate(tab)}
      >
        <span
          className={`h-4 w-0.5 shrink-0 ${tab.active ? 'bg-accent' : 'bg-transparent'}`}
          aria-hidden
        />
        <span className="min-w-0 flex-1 truncate text-xs font-medium">{page}</span>
        {unread != null ? (
          <span className="readout shrink-0 px-1.5 py-0.5 font-mono text-[10px] leading-4 text-lcd">
            {unread}
          </span>
        ) : null}
        {site ? (
          <span className="min-w-0 max-w-[38%] truncate text-[10px] uppercase tracking-[0.06em] text-app-muted">
            {site}
          </span>
        ) : null}
      </button>
      {showToggle ? (
        <button
          type="button"
          aria-label={playing ? 'Pause' : 'Play'}
          className={`flex h-touch w-touch shrink-0 items-center justify-center ${
            playing ? 'text-lcd' : 'text-app-muted'
          }`}
          onClick={(e) => {
            e.stopPropagation();
            onToggleMedia?.(tab);
          }}
        >
          {playing ? (
            <Pause className="h-4 w-4" fill="currentColor" />
          ) : (
            <Play className="h-4 w-4" fill="currentColor" />
          )}
        </button>
      ) : null}
    </div>
  );
}
