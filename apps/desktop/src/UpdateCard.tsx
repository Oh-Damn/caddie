import { useEffect, useState } from 'react';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { Button } from './Button';

type Phase = 'idle' | 'checking' | 'none' | 'found' | 'downloading' | 'ready' | 'error';

// Checked once on mount so a stale build says so without being asked, and the
// check is silent when there is nothing to report: an updater that announces
// "you are up to date" every launch is noise.
export function UpdateCard() {
  const [phase, setPhase] = useState<Phase>('idle');
  const [update, setUpdate] = useState<Update | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const found = await check();
        if (cancelled) return;
        setUpdate(found);
        setPhase(found ? 'found' : 'none');
      } catch {
        // Offline, or GitHub is unreachable. Not worth a message on launch.
        if (!cancelled) setPhase('idle');
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function look() {
    setPhase('checking');
    setError('');
    try {
      const found = await check();
      setUpdate(found);
      setPhase(found ? 'found' : 'none');
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase('error');
    }
  }

  async function install() {
    if (!update) return;
    setPhase('downloading');
    setError('');
    try {
      await update.downloadAndInstall();
      setPhase('ready');
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase('error');
    }
  }

  if (phase === 'idle') return null;

  return (
    <div className="faceplate flex flex-col gap-s2 p-s2">
      {phase === 'found' && update ? (
        <>
          <p className="type-secondary">Caddie {update.version} is available.</p>
          <Button onClick={() => void install()}>Update</Button>
        </>
      ) : null}

      {phase === 'downloading' ? <p className="type-secondary">Downloading…</p> : null}

      {phase === 'ready' ? (
        <>
          <p className="type-secondary">
            Update installed. Caddie needs to restart to use it.
          </p>
          <p className="type-secondary text-app-muted">
            macOS may ask for Accessibility and Automation again afterwards.
          </p>
          <Button onClick={() => void relaunch()}>Restart</Button>
        </>
      ) : null}

      {phase === 'checking' ? <p className="type-secondary">Checking…</p> : null}

      {phase === 'none' ? (
        <>
          <p className="type-secondary text-app-muted">Caddie is up to date.</p>
          <Button variant="ghost" onClick={() => void look()}>
            Check again
          </Button>
        </>
      ) : null}

      {phase === 'error' ? (
        <>
          <p className="type-secondary">Could not check for updates.</p>
          <p className="type-code break-all text-app-muted">{error}</p>
          <Button variant="ghost" onClick={() => void look()}>
            Try again
          </Button>
        </>
      ) : null}
    </div>
  );
}
