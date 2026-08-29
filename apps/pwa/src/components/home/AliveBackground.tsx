import { artworkSrc } from '../../lib/imageColors';

type Props = {
  active: boolean;
  playing: boolean;
  artworkUrl?: string;
};

export function AliveBackground({ active, playing, artworkUrl }: Props) {
  if (!active) return null;

  return (
    <div className={`alive-bg ${playing ? 'alive-bg--playing' : ''}`} aria-hidden>
      <span className="alive-bg__orb alive-bg__orb--a" />
      <span className="alive-bg__orb alive-bg__orb--b" />
      {artworkUrl ? (
        <div className="alive-bg__art">
          <img src={artworkSrc(artworkUrl)} alt="" />
        </div>
      ) : null}
      <div className="alive-bg__scrim" />
      <div className="alive-bg__top" />
      {playing ? <div className="alive-bg__noise" /> : null}
    </div>
  );
}
