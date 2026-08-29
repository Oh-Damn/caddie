import type { NowPlaying } from '@companion/protocol';
import { AppIcon } from '../apps/AppIcon';
import { AlbumArt } from './AlbumArt';

type Props = {
  nowPlaying: NowPlaying;
  sourceName: string;
  sourceBundleId: string;
  onFocus: () => void;
};

export function AlbumHero({ nowPlaying, sourceName, sourceBundleId, onFocus }: Props) {
  const iconClass = 'h-full w-full bg-app-well object-cover';

  return (
    <button
      type="button"
      className="relative min-h-32 w-full flex-1 overflow-hidden bg-app-well text-left landscape:hidden"
      onClick={onFocus}
      aria-label={nowPlaying.title ? `${nowPlaying.title} artwork` : 'Album artwork'}
    >
      {nowPlaying.artworkUrl ? (
        <AlbumArt
          url={nowPlaying.artworkUrl}
          alt=""
          className={iconClass}
          fallback={
            <div className="flex h-full w-full items-center justify-center bg-app-well">
              <AppIcon
                bundleId={sourceBundleId}
                name={sourceName}
                className="h-24 w-24 rounded-app object-cover"
              />
            </div>
          }
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center bg-[repeating-linear-gradient(-45deg,var(--bg-well)_0,var(--bg-well)_12px,var(--border)_12px,var(--border)_13px)]">
          <span className="type-micro text-app-muted">Album art</span>
        </div>
      )}
    </button>
  );
}
