import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import { Button } from './Button';

export type LogLine = {
  ts: string;
  level: string;
  target: string;
  message: string;
  kind: string;
};

type Snapshot = {
  lines: LogLine[];
  crash: string | null;
};

function levelClass(line: LogLine): string {
  if (line.kind === 'panic' || line.kind === 'client' || line.level === 'ERROR') {
    return 'text-danger';
  }
  if (line.level === 'WARN') {
    return 'text-accent';
  }
  return 'text-app-muted';
}

function formatLine(line: LogLine): string {
  return `${line.ts} ${line.level} ${line.target}  ${line.message}`;
}

export function DevLogs() {
  const [lines, setLines] = useState<LogLine[]>([]);
  const [crash, setCrash] = useState<string | null>(null);
  const bottom = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let gone = false;
    const ready = Promise.all([
      invoke<Snapshot>('dev_logs').then((snap) => {
        if (!gone) {
          setLines(snap.lines);
          setCrash(snap.crash);
        }
      }),
      listen<LogLine>('dev-log', (event) => {
        if (gone) {
          return;
        }
        setLines((prev) => [...prev, event.payload].slice(-500));
        if (event.payload.kind === 'panic') {
          setCrash(event.payload.message);
        }
      }),
      listen<string>('dev-crash', (event) => {
        if (!gone) {
          setCrash(event.payload);
        }
      }),
    ]);
    return () => {
      gone = true;
      void ready.then(([, stopLog, stopCrash]) => {
        stopLog();
        stopCrash();
      });
    };
  }, []);

  useEffect(() => {
    bottom.current?.scrollIntoView({ block: 'end' });
  }, [lines]);

  const copy = async () => {
    const text = [crash ? `CRASH ${crash}` : '', ...lines.map(formatLine)]
      .filter(Boolean)
      .join('\n');
    await navigator.clipboard.writeText(text);
  };

  const clear = async () => {
    await invoke('dev_logs_clear');
    setLines([]);
    setCrash(null);
  };

  return (
    <div className="flex min-h-screen flex-col bg-app text-app-text">
      <div className="flex items-center justify-between gap-3 border-b border-app-border px-5 py-4">
        <div>
          <p className="type-micro">Debug</p>
          <h1 className="type-primary">Logs</h1>
        </div>
        <div className="flex gap-2">
          <Button variant="ghost" onClick={() => void copy()}>
            Copy
          </Button>
          <Button variant="ghost" onClick={() => void clear()}>
            Clear
          </Button>
        </div>
      </div>
      {crash ? (
        <div className="border-b border-app-border bg-app-elevated px-5 py-3">
          <p className="type-micro text-danger">Crash</p>
          <p className="type-secondary mt-1 whitespace-pre-wrap text-danger">{crash}</p>
        </div>
      ) : null}
      <div className="flex-1 overflow-auto px-5 py-3 font-mono text-xs leading-5">
        {lines.length === 0 ? (
          <p className="type-meta">No log lines yet.</p>
        ) : (
          lines.map((line, i) => (
            <p key={`${line.ts}-${i}`} className={`break-all ${levelClass(line)}`}>
              {formatLine(line)}
            </p>
          ))
        )}
        <div ref={bottom} />
      </div>
    </div>
  );
}
