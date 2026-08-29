import { useEffect, useState, type ReactNode } from 'react';
import { artworkSrc } from '../../lib/imageColors';

type Props = {
  url: string;
  alt: string;
  className?: string;
  fallback: ReactNode;
};

export function AlbumArt({ url, alt, className, fallback }: Props) {
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setFailed(false);
  }, [url]);

  const frame =
    className ??
    'h-auto w-full rounded-app object-cover border border-app-border bg-app-elevated';

  if (!url || failed) {
    return fallback;
  }

  return (
    <img
      src={artworkSrc(url)}
      alt={alt}
      className={frame}
      referrerPolicy="no-referrer"
      onError={() => setFailed(true)}
    />
  );
}
