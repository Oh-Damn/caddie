import { useEffect, useRef, useState } from 'react';

type Props = {
  title: string;
  meta: string;
  playing?: boolean;
  onClick?: () => void;
  className?: string;
};

export function TrackReadout({ title, meta, playing, onClick, className = '' }: Props) {
  const label = [title, meta].filter(Boolean).join(', ');
  const inner = (
    <>
      {playing != null ? (
        <span
          className={`led relative z-1 ${playing ? 'led-pulse' : 'opacity-30'}`}
          style={{
            color: playing ? 'var(--accent)' : 'var(--border)',
            background: playing ? 'var(--accent)' : 'var(--border)',
          }}
          aria-hidden
        />
      ) : null}
      <div className="relative z-1 flex min-w-0 flex-1 flex-col">
        <MarqueeLine text={title || 'Unknown track'} />
        {meta ? <p className="type-meta mt-0.5 truncate">{meta}</p> : null}
      </div>
    </>
  );
  const look = `readout flex min-w-0 items-center gap-2 px-2 py-1.5 text-left ${className}`;

  if (onClick) {
    return (
      <button type="button" aria-label={label} className={look} onClick={onClick}>
        {inner}
      </button>
    );
  }

  return <div className={look}>{inner}</div>;
}

function motionOk() {
  if (document.documentElement.classList.contains('motion-off')) return false;
  return !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

function MarqueeLine({ text }: { text: string }) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef<HTMLSpanElement>(null);
  const [run, setRun] = useState(false);

  useEffect(() => {
    const wrap = wrapRef.current;
    const measure = measureRef.current;
    if (!wrap || !measure) return;

    const check = () => {
      setRun(motionOk() && measure.scrollWidth > wrap.clientWidth + 1);
    };
    check();
    const ro = new ResizeObserver(check);
    ro.observe(wrap);
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    mq.addEventListener('change', check);
    return () => {
      ro.disconnect();
      mq.removeEventListener('change', check);
    };
  }, [text]);

  return (
    <div ref={wrapRef} className="relative overflow-hidden">
      <span
        ref={measureRef}
        className="invisible pointer-events-none absolute left-0 top-0 whitespace-nowrap font-mono text-[10px] tracking-wide"
      >
        {text}
      </span>
      {run ? (
        <div className="lcd-marquee" aria-hidden>
          <span className="lcd-marquee__item">{text}</span>
          <span className="lcd-marquee__item">{text}</span>
        </div>
      ) : (
        <span className="block truncate font-mono text-[10px] tracking-wide text-lcd">
          {text}
        </span>
      )}
    </div>
  );
}
