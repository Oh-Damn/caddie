import { PRODUCT_NAME } from '@companion/protocol';
import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { Button } from './Button';

const STEPS = ['accessibility', 'automation'] as const;
type Step = (typeof STEPS)[number];

type Props = {
  onDone: () => void;
  accessibilityTrusted: boolean;
};

export function Onboarding({ onDone, accessibilityTrusted }: Props) {
  const [step, setStep] = useState<Step>('accessibility');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const idx = STEPS.indexOf(step);

  const go = (delta: number) => {
    const n = idx + delta;
    if (n >= STEPS.length) {
      onDone();
      return;
    }
    setStep(STEPS[Math.max(0, n)]);
  };

  const run = async (command: string) => {
    setBusy(true);
    setError(null);
    try {
      await invoke(command);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-screen flex-col bg-app text-app-text">
      <header className="flex shrink-0 items-center gap-s3 border-b border-app-border bg-app-elevated px-s3 py-s3">
        <p className="type-primary">{PRODUCT_NAME}</p>
        <span
          className="flex flex-1 gap-1"
          aria-label={`Step ${idx + 1} of ${STEPS.length}`}
        >
          {STEPS.map((name, i) => (
            <span
              key={name}
              className={`h-0.5 flex-1 ${i <= idx ? 'bg-accent' : 'bg-app-border'}`}
            />
          ))}
        </span>
      </header>

      <div className="flex min-h-0 flex-1 flex-col gap-s3 overflow-y-auto px-s3 py-s4">
        {step === 'accessibility' ? (
          <>
            <h1 className="type-display">Accessibility</h1>
            <p className="type-secondary text-app-muted">
              {PRODUCT_NAME} presses keys on your behalf. Shortcuts, media keys, paste.
              macOS keeps that behind a switch you have to flip yourself.
            </p>

            <div className="readout flex flex-1 flex-col items-center justify-center gap-s2 py-s5">
              <span
                className="led"
                style={{
                  color: accessibilityTrusted
                    ? 'var(--status-ready)'
                    : 'var(--status-wait)',
                  background: accessibilityTrusted
                    ? 'var(--status-ready)'
                    : 'var(--status-wait)',
                }}
                aria-hidden
              />
              <p className="type-stat">
                {accessibilityTrusted ? 'GRANTED' : 'NOT GRANTED'}
              </p>
              <p className="type-micro">
                {accessibilityTrusted ? 'Nothing else to do here' : 'Reads live'}
              </p>
            </div>

            <p className="type-secondary text-app-muted">
              Privacy and Security, then Accessibility. Find {PRODUCT_NAME} in the list
              and switch it on.
            </p>
            <Button onClick={() => void run('request_accessibility')} disabled={busy}>
              Open Settings
            </Button>
          </>
        ) : (
          <>
            <h1 className="type-display">Automation</h1>
            <p className="type-secondary text-app-muted">
              Reading the frontmost app and what is playing goes through Automation. macOS
              asks once per app, and you can answer as the prompts arrive.
            </p>

            <dl className="faceplate flex flex-col">
              {(
                [
                  ['System Events', 'Required'],
                  ['Music, Spotify', 'Now playing'],
                  ['Safari, Chrome, Brave, Arc', 'Tab titles'],
                ] as const
              ).map(([term, detail]) => (
                <div
                  key={term}
                  className="flex items-baseline justify-between gap-s2 border-b border-app-border px-s2 py-s2 last:border-b-0"
                >
                  <dt className="type-secondary">{term}</dt>
                  <dd className="type-micro shrink-0">{detail}</dd>
                </div>
              ))}
            </dl>

            <Button onClick={() => void run('request_automation')} disabled={busy}>
              Prompt now
            </Button>
            <p className="type-secondary text-app-muted">
              Skip it and macOS asks the first time each app is needed. Your phone also
              has to be on this Wi-Fi network, and macOS may ask to allow that too.
            </p>
          </>
        )}

        {error ? <p className="type-secondary text-danger">{error}</p> : null}
      </div>

      <div className="transport flex shrink-0 items-center justify-between gap-s2 px-s3 py-s2">
        <Button variant="ghost" onClick={() => go(-1)} disabled={busy || idx === 0}>
          Back
        </Button>
        <Button onClick={() => go(1)} disabled={busy}>
          {idx === STEPS.length - 1 ? 'Pair phone' : 'Continue'}
        </Button>
      </div>
    </div>
  );
}
