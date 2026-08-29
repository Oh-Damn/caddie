import type { CSSProperties } from 'react';

type Props = {
  className?: string;
  style?: CSSProperties;
};

export function Skeleton({ className = '', style }: Props) {
  return <div className={`skeleton ${className}`.trim()} style={style} aria-hidden />;
}
