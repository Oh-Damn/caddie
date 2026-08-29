import { Minus, Plus, Volume2, VolumeX } from 'lucide-react';
import { useEffect, useRef, useState, type PointerEvent } from 'react';
import { IconButton } from '../ui/IconButton';
import { MediaSources, type MediaSource } from './MediaSources';

type Props = {
  sources: MediaSource[];
  volume: number;
  muted: boolean;
  showMediaControls?: boolean;
  compact?: boolean;
  hideReadoutFor?: string;
  onCommand: (action: string, value?: number | null) => void;
  onMediaCommand: (action: string, value: number | null, target: string) => void;
  onFocusApp: (bundleId: string) => void;
};

export function MediaVolumeBar({
  sources,
  volume,
  muted,
  showMediaControls = true,
  compact = false,
  hideReadoutFor,
  onCommand,
  onMediaCommand,
  onFocusApp,
}: Props) {
  const level = Math.round(volume);
  const showMedia = showMediaControls && sources.length > 0;
  const VolIcon = muted ? VolumeX : Volume2;

  const row = (
    <div className="flex min-w-0 flex-1 items-center gap-1.5">
      <button
        type="button"
        className={`flex h-8 w-8 shrink-0 items-center justify-center ${
          muted ? 'key key-latch' : 'key text-app-muted'
        }`}
        aria-label={muted ? 'Unmute' : 'Mute'}
        onClick={() => onCommand('volume.toggle_mute')}
      >
        <VolIcon className="h-4 w-4" strokeWidth={2} />
      </button>
      <IconButton
        label="Volume down"
        variant="chip"
        size="sm"
        className={compact ? '' : 'landscape:order-1'}
        onClick={() => onCommand('volume.down')}
      >
        <Minus className="h-3.5 w-3.5" strokeWidth={2.5} />
      </IconButton>
      <VolumeTrack
        level={level}
        muted={muted}
        className={compact ? '' : 'landscape:hidden'}
        onSet={(value) => onCommand('volume.set', value)}
      />
      <IconButton
        label="Volume up"
        variant="chip"
        size="sm"
        accent
        className={compact ? '' : 'landscape:order-3'}
        onClick={() => onCommand('volume.up')}
      >
        <Plus className="h-3.5 w-3.5" strokeWidth={2.5} />
      </IconButton>
      <VolumeReadout
        level={level}
        muted={muted}
        className={
          compact
            ? ''
            : 'landscape:order-2 landscape:min-w-0 landscape:w-auto landscape:flex-1'
        }
        onSet={(value) => onCommand('volume.set', value)}
      />
    </div>
  );

  if (compact) {
    return row;
  }

  return (
    <div className="faceplate flex shrink-0 flex-col overflow-hidden landscape:h-full landscape:min-h-0 landscape:flex-1">
      {sources.length > 0 ? (
        <div
          className={`flex min-h-0 flex-col ${showMedia ? '' : 'portrait:hidden'} landscape:min-h-0 landscape:flex-1`}
        >
          <MediaSources
            sources={sources}
            hideReadoutFor={hideReadoutFor}
            onCommand={onMediaCommand}
            onFocusApp={onFocusApp}
          />
          <div className="h-px shrink-0 bg-app-border" />
        </div>
      ) : null}

      <div className="flex shrink-0 items-center px-2 py-2 landscape:px-2 landscape:py-1.5">
        {row}
      </div>
    </div>
  );
}

function VolumeTrack({
  level,
  muted,
  onSet,
  className = '',
}: {
  level: number;
  muted: boolean;
  onSet: (value: number) => void;
  className?: string;
}) {
  const ref = useRef<HTMLButtonElement>(null);
  const dragging = useRef(false);

  const fromX = (clientX: number) => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const pct = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    onSet(Math.round(pct * 100));
  };

  const onPointerDown = (e: PointerEvent<HTMLButtonElement>) => {
    dragging.current = true;
    e.currentTarget.setPointerCapture(e.pointerId);
    fromX(e.clientX);
  };

  const onPointerMove = (e: PointerEvent<HTMLButtonElement>) => {
    if (dragging.current) fromX(e.clientX);
  };

  const stop = () => {
    dragging.current = false;
  };

  return (
    <button
      ref={ref}
      type="button"
      className={`slider-track min-w-0 flex-1 ${className}`}
      aria-label={`Volume ${muted ? `${level}, muted` : level}`}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={level}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={stop}
      onPointerCancel={stop}
    >
      <span
        className={`slider-fill ${muted ? 'opacity-30' : ''}`}
        style={{ width: `calc(${level}% - 4px)` }}
      />
      <span className="slider-thumb" style={{ left: `${level}%` }} />
    </button>
  );
}

function VolumeReadout({
  level,
  muted,
  onSet,
  className = '',
}: {
  level: number;
  muted: boolean;
  onSet: (value: number) => void;
  className?: string;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(String(level));
  const inputRef = useRef<HTMLInputElement>(null);
  const digits = String(level).padStart(3, '0');

  useEffect(() => {
    if (!editing) setDraft(String(level));
  }, [editing, level]);

  useEffect(() => {
    if (editing) inputRef.current?.focus();
  }, [editing]);

  const commit = () => {
    const raw = draft.trim();
    setEditing(false);
    if (raw === '') {
      setDraft(String(level));
      return;
    }
    const n = Number.parseInt(raw, 10);
    if (!Number.isFinite(n)) {
      setDraft(String(level));
      return;
    }
    onSet(Math.max(0, Math.min(100, n)));
  };

  if (editing) {
    return (
      <form
        className={`readout relative z-[1] h-8 w-12 shrink-0 ${className}`}
        onSubmit={(e) => {
          e.preventDefault();
          commit();
        }}
      >
        <input
          ref={inputRef}
          value={draft}
          inputMode="numeric"
          pattern="[0-9]*"
          enterKeyHint="done"
          maxLength={3}
          aria-label="Volume"
          className="relative z-[1] h-full w-full bg-transparent px-1 text-center font-mono text-base tabular-nums text-lcd"
          onChange={(e) => setDraft(e.target.value.replace(/\D/g, '').slice(0, 3))}
          onBlur={commit}
        />
      </form>
    );
  }

  return (
    <button
      type="button"
      aria-label={`Volume ${muted ? `${level}, muted` : level}. Tap to type`}
      className={`readout relative z-[1] h-8 w-12 shrink-0 px-1 text-center font-mono text-base tabular-nums ${
        muted ? 'text-danger line-through' : 'text-lcd'
      } ${className}`}
      onClick={() => {
        setDraft(String(level));
        setEditing(true);
      }}
    >
      {digits}
    </button>
  );
}
