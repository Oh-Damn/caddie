import type { NowPlaying } from '@companion/protocol';
import { AlbumHero } from './AlbumHero';
import { MediaControls } from './MediaControls';
import { ProgressBar } from './ProgressBar';

type Props = {
  nowPlaying: NowPlaying;
  sourceName: string;
  sourceBundleId: string;
  onFocus: () => void;
  onCommand: (action: string) => void;
  connected: boolean;
};

export function MediaPanel({
  nowPlaying,
  sourceName,
  sourceBundleId,
  onFocus,
  onCommand,
  connected,
}: Props) {
  return (
    <div className="faceplate flex min-h-0 flex-col overflow-hidden portrait:flex-1">
      <AlbumHero
        nowPlaying={nowPlaying}
        sourceName={sourceName}
        sourceBundleId={sourceBundleId}
        onFocus={onFocus}
      />
      <div className="transport flex shrink-0 flex-col gap-1.5 px-2 py-1.5">
        <ProgressBar
          positionSec={nowPlaying.positionSec}
          durationSec={nowPlaying.durationSec}
          playing={nowPlaying.playing}
          connected={connected}
          onSeekBack={() => onCommand('media.seek_back')}
          onSeekForward={() => onCommand('media.seek_forward')}
        />
        <MediaControls nowPlaying={nowPlaying} onCommand={onCommand} />
      </div>
    </div>
  );
}
