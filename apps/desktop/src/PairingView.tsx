import type { SessionInfo } from '@companion/protocol';
import { invoke } from '@tauri-apps/api/core';
import { QRCodeSVG } from 'qrcode.react';
import type { ReactNode } from 'react';
import { Button } from './Button';
import { DeviceCard } from './DeviceCard';
import { PoweredBy } from './PoweredBy';
import { ProvidersCard } from './ProvidersCard';

type Props = {
  session: SessionInfo;
  error: string | null;
};

export function PairingView({ session, error }: Props) {
  const connected = session.clientCount > 0;

  return (
    <div className="flex h-screen flex-col bg-app text-app-text">
      <div className="h-1 shrink-0" style={{ background: 'var(--accent)' }} aria-hidden />

      <header className="flex shrink-0 items-center justify-between gap-s2 border-b border-app-border bg-app-elevated px-s3 py-s3">
        <p className="type-primary">Caddie</p>
        <p className="flex items-center gap-s2 type-micro">
          <span
            className="led"
            style={{
              color: connected ? 'var(--status-ready)' : 'var(--status-wait)',
              background: connected ? 'var(--status-ready)' : 'var(--status-wait)',
            }}
            aria-hidden
          />
          {connected ? 'Connected' : 'Waiting for a phone'}
        </p>
      </header>

      <div className="flex min-h-0 flex-1 flex-col gap-s3 overflow-y-auto px-s3 py-s3">
        {error ? <p className="type-secondary text-danger">{error}</p> : null}

        {connected ? (
          <>
            <DeviceCard device={session.device} connected />
            <div className="faceplate p-s2">
              <p className="type-micro">Frontmost app</p>
              <p className="type-secondary mt-1">
                {session.live
                  ? session.appName || 'Desktop'
                  : 'Idle until your phone is open'}
              </p>
            </div>
          </>
        ) : (
          <>
            <Step n={1} label="Scan">
              <p className="type-secondary text-app-muted">
                Point your phone camera at this. Both devices need to be on the same
                Wi-Fi.
              </p>
              <div
                className="self-center p-[3px]"
                style={{
                  background: 'var(--accent)',
                  borderRadius: 'var(--radius)',
                }}
              >
                <div className="paper-well p-s2">
                  <QRCodeSVG value={session.httpUrl} size={196} />
                </div>
              </div>
            </Step>

            <Step n={2} label="Or type it in">
              <div className="flex items-baseline justify-between gap-s2">
                <span className="type-micro shrink-0">Address</span>
                <span className="type-code break-all text-right">
                  {hostOnly(session.httpUrl)}
                </span>
              </div>
              <div
                className="readout flex items-baseline justify-between gap-s2 px-s2 py-2"
                style={{ color: 'var(--accent)' }}
              >
                <span className="type-micro" style={{ color: 'var(--text-muted)' }}>
                  Code
                </span>
                <span className="type-stat" style={{ letterSpacing: '0.22em' }}>
                  {session.pairingSecret}
                </span>
              </div>
            </Step>
          </>
        )}

        {!session.accessibilityTrusted ? (
          <div className="faceplate flex flex-col gap-s2 p-s2">
            <p className="type-secondary">
              Shortcuts and media keys stay dead until Accessibility is on.
            </p>
            <Button
              variant="ghost"
              onClick={() => {
                void invoke('request_accessibility');
              }}
            >
              Open Settings
            </Button>
          </div>
        ) : null}

        <ProvidersCard />

        <details className="faceplate px-s2 py-s2">
          <summary className="type-micro cursor-pointer">Phone will not connect</summary>
          <div className="mt-s2 flex flex-col gap-s2">
            <p className="type-secondary text-app-muted">
              A certificate warning is expected. This Mac signs its own, because the
              connection never leaves your network. Tap Advanced, then Continue.
            </p>
            <p className="type-secondary text-app-muted">
              If the .local name does not resolve, use the address by number:
            </p>
            <p className="type-code break-all">{hostOnly(session.fallbackHttpUrl)}</p>
            <p className="type-code text-app-muted">
              server {session.fingerprint} / cert {session.certFingerprint}
            </p>
          </div>
        </details>

        <PoweredBy />
      </div>
    </div>
  );
}

function Step({ n, label, children }: { n: number; label: string; children: ReactNode }) {
  return (
    <section className="faceplate flex flex-col gap-s2 p-s2">
      <p className="flex items-center gap-s2">
        <span
          className="flex h-5 w-5 shrink-0 items-center justify-center type-micro"
          style={{
            background: 'var(--accent)',
            color: 'var(--accent-text)',
            borderRadius: 'var(--radius)',
          }}
          aria-hidden
        >
          {n}
        </span>
        <span className="type-micro">{label}</span>
      </p>
      {children}
    </section>
  );
}

function hostOnly(url: string): string {
  return url
    .replace(/^https?:\/\//, '')
    .split('?')[0]
    .replace(/\/$/, '');
}
