import type { NowPlaying } from '@companion/protocol';
import { Pause, Play, SkipBack, SkipForward } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { AlbumArt } from './AlbumArt';
import { IconButton } from '../ui/IconButton';
import { TrackReadout } from './TrackReadout';

export type MediaSource = NowPlaying & { controllable: boolean };

type Props = {
  sources: MediaSource[];
  onCommand: (action: string, value: number | null, target: string) => void;
  onFocusApp: (bundleId: string) => void;
  hideReadoutFor?: string;
};

const NO_BAR = '[scrollbar-width:none] [&::-webkit-scrollbar]:hidden';

export function MediaSources({ sources, onCommand, onFocusApp, hideReadoutFor }: Props) {
  const trackRef = useRef<HTMLDivElement>(null);
  const [index, setIndex] = useState(0);
  const touchedRef = useRef(false);

  const playingIndex = sources.findIndex((s) => s.playing);

  useEffect(() => {
    if (touchedRef.current || playingIndex < 0 || playingIndex === index) return;
    setIndex(playingIndex);
    const el = trackRef.current;
    if (el) el.scrollTo({ left: playingIndex * el.clientWidth, behavior: 'auto' });
  }, [index, playingIndex]);

  useEffect(() => {
    if (index < sources.length) return;
    setIndex(Math.max(0, sources.length - 1));
  }, [index, sources.length]);

  if (sources.length === 0) return null;

  const onScroll = () => {
    const el = trackRef.current;
    if (!el || el.clientWidth === 0) return;
    const next = Math.round(el.scrollLeft / el.clientWidth);
    if (next !== index) setIndex(next);
  };

  return (
    <div className="flex min-h-0 min-w-0 flex-col landscape:h-full landscape:min-h-0 landscape:flex-1">
      <div
        ref={trackRef}
        onScroll={onScroll}
        onPointerDown={() => {
          touchedRef.current = true;
        }}
        className={`flex w-full min-w-0 snap-x snap-mandatory overflow-x-auto overflow-y-hidden landscape:min-h-0 landscape:flex-1 ${NO_BAR}`}
      >
        {sources.map((source) => (
          <SourceCard
            key={source.sourceBundleId || source.sourceName}
            source={source}
            hideReadout={Boolean(
              hideReadoutFor && source.sourceBundleId === hideReadoutFor,
            )}
            onCommand={onCommand}
            onFocusApp={onFocusApp}
          />
        ))}
      </div>
      {sources.length > 1 ? (
        <div className="flex shrink-0 items-center justify-center gap-1 pb-1.5 landscape:px-2 landscape:pb-1">
          {sources.map((source, i) => (
            <span
              key={source.sourceBundleId || source.sourceName}
              aria-hidden
              className={`led ${i === index ? '' : 'opacity-30'}`}
              style={{
                color: i === index ? 'var(--accent)' : 'var(--border)',
                background: i === index ? 'var(--accent)' : 'var(--border)',
              }}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function SourceCard({
  source,
  hideReadout = false,
  onCommand,
  onFocusApp,
}: {
  source: MediaSource;
  hideReadout?: boolean;
  onCommand: (action: string, value: number | null, target: string) => void;
  onFocusApp: (bundleId: string) => void;
}) {
  const target = source.sourceBundleId;
  const go = (action: string) => onCommand(action, null, target);
  const meta = source.controllable
    ? [source.artist, source.sourceName].filter(Boolean).join(' \u00b7 ')
    : `${source.sourceName} \u00b7 not the system source`;

  return (
    <div className="flex w-full shrink-0 snap-center items-center gap-2 px-2.5 py-2 landscape:h-full landscape:min-h-0 landscape:flex-col landscape:items-stretch landscape:justify-start landscape:gap-0 landscape:px-0 landscape:py-0">
      <div className="relative h-9 w-9 shrink-0 overflow-hidden rounded-app bg-app-well landscape:h-auto landscape:min-h-0 landscape:w-full landscape:flex-1 landscape:rounded-none">
        <button
          type="button"
          disabled={!source.controllable}
          className="absolute inset-0 disabled:opacity-50"
          aria-label={source.playing ? 'Pause' : 'Play'}
          onClick={() => go('media.play_pause')}
        >
          {source.artworkUrl ? (
            <AlbumArt
              url={source.artworkUrl}
              alt=""
              className="h-full w-full object-cover"
              fallback={<span className="block h-full w-full bg-accent/25" />}
            />
          ) : (
            <span className="block h-full w-full bg-accent/25" />
          )}
          <span className="absolute inset-0 flex items-center justify-center bg-black/30">
            {source.playing ? (
              <Pause
                className="h-4 w-4 text-white landscape:h-5 landscape:w-5"
                fill="currentColor"
              />
            ) : (
              <Play
                className="h-4 w-4 text-white landscape:h-5 landscape:w-5"
                fill="currentColor"
              />
            )}
          </span>
        </button>
        {source.controllable ? (
          <div className="pointer-events-none absolute inset-x-0 bottom-2 z-10 hidden justify-between px-1 landscape:flex">
            <span className="pointer-events-auto">
              <IconButton label="Previous" size="sm" onClick={() => go('media.prev')}>
                <SkipBack className="h-3.5 w-3.5 text-white" fill="currentColor" />
              </IconButton>
            </span>
            <span className="pointer-events-auto">
              <IconButton label="Next" size="sm" onClick={() => go('media.next')}>
                <SkipForward className="h-3.5 w-3.5 text-white" fill="currentColor" />
              </IconButton>
            </span>
          </div>
        ) : null}
      </div>
      <TrackReadout
        title={source.title || 'Unknown track'}
        meta={meta}
        playing={source.playing}
        onClick={() => {
          if (target) onFocusApp(target);
        }}
        className={`min-w-0 flex-1 landscape:mx-1.5 landscape:my-1.5 landscape:flex-none ${
          hideReadout ? 'landscape:hidden' : ''
        }`}
      />
      {source.controllable ? (
        <div className="flex items-center justify-center gap-1 landscape:hidden">
          <IconButton label="Previous" onClick={() => go('media.prev')}>
            <SkipBack className="h-4 w-4" fill="currentColor" />
          </IconButton>
          <IconButton label="Next" onClick={() => go('media.next')}>
            <SkipForward className="h-4 w-4" fill="currentColor" />
          </IconButton>
        </div>
      ) : null}
    </div>
  );
}
