import { useLivePosition } from '../../companion/useLivePosition';
import { formatTime } from '../../lib/formatTime';

type Props = {
  positionSec: number;
  durationSec: number;
  playing: boolean;
  connected: boolean;
  onSeekBack?: () => void;
  onSeekForward?: () => void;
};

export function ProgressBar({
  positionSec,
  durationSec,
  playing,
  connected,
  onSeekBack,
  onSeekForward,
}: Props) {
  const live = useLivePosition(positionSec, durationSec, playing, connected);
  if (durationSec <= 0) return null;
  const pct = Math.min(100, Math.max(0, (live / durationSec) * 100));

  return (
    <div className="flex shrink-0 items-center gap-2">
      <TimeSeek
        time={formatTime(live)}
        hint="-15"
        label="Rewind 15 seconds"
        onSeek={onSeekBack}
      />
      <div className="progress-seg min-w-0 flex-1">
        <span style={{ width: `${pct}%` }} />
      </div>
      <TimeSeek
        time={formatTime(durationSec)}
        hint="+15"
        label="Forward 15 seconds"
        onSeek={onSeekForward}
      />
    </div>
  );
}

function TimeSeek({
  time,
  hint,
  label,
  onSeek,
}: {
  time: string;
  hint: string;
  label: string;
  onSeek?: () => void;
}) {
  const body = (
    <>
      <span className="font-mono text-[10px] tabular-nums text-lcd">{time}</span>
      {onSeek ? <span className="type-micro">{hint}</span> : null}
    </>
  );

  if (!onSeek) {
    return <span className="flex min-w-10 shrink-0 flex-col items-center">{body}</span>;
  }

  return (
    <button
      type="button"
      aria-label={label}
      className="flex min-w-10 shrink-0 flex-col items-center py-0.5"
      onClick={onSeek}
    >
      {body}
    </button>
  );
}
