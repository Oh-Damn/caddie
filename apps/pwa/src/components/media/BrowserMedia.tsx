import type { NowPlaying } from '@companion/protocol';
import { MediaControls } from './MediaControls';

type Props = {
  nowPlaying: NowPlaying;
  onCommand: (action: string) => void;
};

export function BrowserMedia({ nowPlaying, onCommand }: Props) {
  return (
    <div className="flex flex-col gap-2">
      <div className="px-0.5">
        <p className="truncate font-mono text-xs tracking-wide text-lcd">
          {nowPlaying.title}
        </p>
        <p className="type-meta truncate">
          {nowPlaying.sourceName} · {nowPlaying.playing ? 'Playing' : 'Paused'}
        </p>
      </div>
      <MediaControls nowPlaying={nowPlaying} onCommand={onCommand} />
    </div>
  );
}
