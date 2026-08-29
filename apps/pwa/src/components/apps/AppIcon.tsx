import { useEffect, useState } from 'react';
import { appIconSrc } from '../../lib/appIconSrc';
import { AppMark } from './AppMark';

type Props = {
  bundleId: string;
  name: string;
  className?: string;
};

export function AppIcon({ bundleId, name, className }: Props) {
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    setFailed(false);
  }, [bundleId]);

  const frame =
    className ??
    'h-24 w-24 rounded-app object-cover border border-app-border bg-app-elevated';

  if (!bundleId || failed) {
    return <AppMark bundleId={bundleId} name={name} className={frame} />;
  }

  return (
    <img
      src={appIconSrc(bundleId)}
      alt=""
      className={frame}
      onError={() => setFailed(true)}
    />
  );
}
