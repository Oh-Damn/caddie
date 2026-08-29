import type { SessionInfo } from '@companion/protocol';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState, type ReactNode } from 'react';
import { DevLogs } from './DevLogs';
import { ErrorBoundary } from './ErrorBoundary';
import { Onboarding } from './Onboarding';
import { PairingView } from './PairingView';
import { PoweredBy } from './PoweredBy';
import { UninstallView } from './UninstallView';

function logsView(): boolean {
  return (
    window.__CADDIE_VIEW__ === 'logs' ||
    new URLSearchParams(window.location.search).get('view') === 'logs' ||
    window.location.hash === '#logs'
  );
}

function useDebug() {
  const [debug, setDebug] = useState<boolean | null>(null);

  useEffect(() => {
    void invoke<boolean>('is_debug').then(setDebug);
  }, []);

  useEffect(() => {
    if (debug !== true) {
      return;
    }
    const onError = (event: ErrorEvent) => {
      const message =
        event.error instanceof Error
          ? (event.error.stack ?? event.error.message)
          : event.message;
      void invoke('dev_client_error', { message });
    };
    const onReject = (event: PromiseRejectionEvent) => {
      const reason = event.reason;
      const message =
        reason instanceof Error ? (reason.stack ?? reason.message) : String(reason);
      void invoke('dev_client_error', { message });
    };
    window.addEventListener('error', onError);
    window.addEventListener('unhandledrejection', onReject);
    return () => {
      window.removeEventListener('error', onError);
      window.removeEventListener('unhandledrejection', onReject);
    };
  }, [debug]);

  return debug;
}

function DebugGate({ children }: { children: ReactNode }) {
  const debug = useDebug();
  if (debug !== true) {
    return children;
  }
  return <ErrorBoundary>{children}</ErrorBoundary>;
}

function LogsGate() {
  const debug = useDebug();
  if (debug === null) {
    return <div className="min-h-screen bg-app" />;
  }
  if (!debug) {
    return <div className="min-h-screen bg-app px-6 py-8 text-app-muted">Debug only</div>;
  }
  return <DevLogs />;
}

function PairingApp() {
  const [session, setSession] = useState<SessionInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [onboarded, setOnboarded] = useState<boolean | null>(null);
  const [uninstall, setUninstall] = useState(false);

  useEffect(() => {
    let alive = true;
    const tick = async () => {
      try {
        const next = await invoke<SessionInfo>('session');
        if (!alive) {
          return;
        }
        setSession(next);
        setError(null);
        setOnboarded((prev) => prev ?? next.onboardingComplete);
        if (next.uninstallPrompt) {
          setUninstall(true);
        }
      } catch (e) {
        if (alive) {
          setError(String(e));
        }
      }
    };
    void tick();
    const id = window.setInterval(() => void tick(), 1000);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, []);

  useEffect(() => {
    let gone = false;
    const ready = listen('uninstall-prompt', () => {
      if (!gone) {
        setUninstall(true);
      }
    });
    return () => {
      gone = true;
      void ready.then((stop) => stop());
    };
  }, []);

  const finishOnboarding = async () => {
    try {
      await invoke('complete_onboarding');
      setOnboarded(true);
    } catch (e) {
      setError(String(e));
    }
  };

  const cancelUninstall = async () => {
    try {
      await invoke('cancel_uninstall');
    } catch (e) {
      setError(String(e));
    }
    setUninstall(false);
  };

  if (uninstall) {
    return (
      <UninstallView
        bundled={session?.bundled ?? false}
        onCancel={() => void cancelUninstall()}
      />
    );
  }

  if (onboarded === false) {
    return (
      <Onboarding
        onDone={() => void finishOnboarding()}
        accessibilityTrusted={session?.accessibilityTrusted ?? false}
      />
    );
  }

  if (!session) {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center gap-5 bg-app px-6">
        <PoweredBy variant="startup" />
        <p className="type-secondary text-app-muted">Starting server...</p>
      </div>
    );
  }

  return <PairingView session={session} error={error} />;
}

export default function App() {
  if (logsView()) {
    return <LogsGate />;
  }
  return (
    <DebugGate>
      <PairingApp />
    </DebugGate>
  );
}
