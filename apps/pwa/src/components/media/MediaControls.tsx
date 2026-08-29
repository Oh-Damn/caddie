import type { NowPlaying, RepeatMode } from '@companion/protocol';
import {
  Heart,
  Pause,
  Play,
  Repeat,
  Repeat1,
  Shuffle,
  SkipBack,
  SkipForward,
} from 'lucide-react';
import type { ReactNode } from 'react';

type Props = {
  nowPlaying: NowPlaying;
  onCommand: (action: string) => void;
};

const PLAYER_BUNDLES = new Set(['com.spotify.client', 'com.apple.Music']);

export function MediaControls({ nowPlaying, onCommand }: Props) {
  const player =
    PLAYER_BUNDLES.has(nowPlaying.sourceBundleId) && nowPlaying.durationSec > 0;
  const repeatActive = nowPlaying.repeatMode !== 'off';
  const RepeatIcon = nowPlaying.repeatMode === 'one' ? Repeat1 : Repeat;

  return (
    <div className="flex shrink-0 flex-col gap-1.5">
      {player ? (
        <div className="flex items-stretch gap-1.5">
          <LatchKey
            caption="Shuf"
            label="Shuffle"
            on={nowPlaying.shuffle}
            onClick={() => onCommand('media.shuffle')}
          >
            <Shuffle className="h-3.5 w-3.5" strokeWidth={2.2} />
          </LatchKey>
          <LatchKey
            caption="Like"
            label={nowPlaying.liked ? 'Unlike' : 'Like'}
            on={nowPlaying.liked}
            onClick={() => onCommand('media.like')}
          >
            <Heart
              className="h-3.5 w-3.5"
              fill={nowPlaying.liked ? 'currentColor' : 'none'}
              strokeWidth={2.2}
            />
          </LatchKey>
          <LatchKey
            caption={nowPlaying.repeatMode === 'one' ? 'One' : 'Rpt'}
            label={repeatLabel(nowPlaying.repeatMode)}
            on={repeatActive}
            onClick={() => onCommand('media.repeat')}
          >
            <RepeatIcon className="h-3.5 w-3.5" strokeWidth={2.2} />
          </LatchKey>
        </div>
      ) : null}

      <div className="flex items-stretch gap-1.5">
        <button
          type="button"
          className="key flex h-12 w-12 shrink-0 items-center justify-center text-app-muted"
          aria-label="Previous"
          onClick={() => onCommand('media.prev')}
        >
          <SkipBack className="h-5 w-5" fill="currentColor" />
        </button>
        <button
          type="button"
          className="key key-lit flex h-12 min-w-12 flex-1 items-center justify-center"
          aria-label={nowPlaying.playing ? 'Pause' : 'Play'}
          onClick={() => onCommand('media.play_pause')}
        >
          {nowPlaying.playing ? (
            <Pause className="h-6 w-6" fill="currentColor" />
          ) : (
            <Play className="h-6 w-6" fill="currentColor" />
          )}
        </button>
        <button
          type="button"
          className="key flex h-12 w-12 shrink-0 items-center justify-center text-app-muted"
          aria-label="Next"
          onClick={() => onCommand('media.next')}
        >
          <SkipForward className="h-5 w-5" fill="currentColor" />
        </button>
      </div>
    </div>
  );
}

function LatchKey({
  caption,
  label,
  on,
  onClick,
  children,
}: {
  caption: string;
  label: string;
  on: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  const led = on ? 'var(--accent)' : 'var(--border)';

  return (
    <button
      type="button"
      className="key flex min-w-0 flex-1 flex-col items-center gap-0.5 px-1 py-1 text-app-muted"
      aria-label={label}
      aria-pressed={on}
      onClick={onClick}
    >
      <span className="flex items-center gap-1">
        <span className="led" style={{ color: led, background: led }} aria-hidden />
        {children}
      </span>
      <span className="type-micro">{caption}</span>
    </button>
  );
}

function repeatLabel(mode: RepeatMode): string {
  if (mode === 'one') return 'Repeat one';
  if (mode === 'all') return 'Repeat all';
  return 'Repeat off';
}
