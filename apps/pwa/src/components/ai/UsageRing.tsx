import type { ReactNode } from 'react';
import { TONE_VAR, type Tone } from '../../companion/agents';

const R = 38;
const C = 2 * Math.PI * R;

type Props = {
  percent: number;
  tone: Tone;
  label: string;
  children: ReactNode;
};

export function UsageRing({ percent, tone, label, children }: Props) {
  const clamped = Math.min(100, Math.max(0, percent));
  const dash = (clamped / 100) * C;

  return (
    <div className="relative grid h-22 w-22 place-items-center">
      <svg viewBox="0 0 100 100" className="h-full w-full" role="img" aria-label={label}>
        <circle
          cx="50"
          cy="50"
          r={R}
          fill="none"
          stroke="var(--highlight)"
          strokeWidth="8"
        />
        {clamped > 0 ? (
          <circle
            cx="50"
            cy="50"
            r={R}
            fill="none"
            className="usage-ring__arc"
            stroke={TONE_VAR[tone]}
            strokeWidth="8"
            strokeLinecap="round"
            strokeDasharray={`${dash} ${C}`}
            transform="rotate(-90 50 50)"
          />
        ) : null}
      </svg>
      <div className="absolute inset-0 grid place-items-center">{children}</div>
    </div>
  );
}
