import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

type Provider = {
  id: string;
  name: string;
  enabled: boolean;
  reads: string[];
};

export function ProvidersCard() {
  const [providers, setProviders] = useState<Provider[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    void invoke<Provider[]>('providers')
      .then((next) => {
        if (alive) {
          setProviders(next);
        }
      })
      .catch((e) => {
        if (alive) {
          setError(String(e));
        }
      });
    return () => {
      alive = false;
    };
  }, []);

  const toggle = async (id: string, next: boolean) => {
    setProviders(
      (prev) => prev?.map((p) => (p.id === id ? { ...p, enabled: next } : p)) ?? prev,
    );
    setError(null);
    try {
      await invoke('set_provider_enabled', { id, enabled: next });
    } catch (e) {
      setError(String(e));
      setProviders(
        (prev) => prev?.map((p) => (p.id === id ? { ...p, enabled: !next } : p)) ?? prev,
      );
    }
  };

  if (!providers) {
    return null;
  }

  return (
    <details className="faceplate px-s2 py-s2">
      <summary className="type-micro cursor-pointer">
        Agents ({providers.filter((p) => p.enabled).length} of {providers.length} on)
      </summary>
      <div className="mt-s2 flex flex-col gap-s2">
        <p className="type-secondary text-app-muted">
          Off means Caddie never reads that source. Nothing is collected in the background
          and the card disappears from your phone.
        </p>
        {error ? <p className="type-secondary text-danger">{error}</p> : null}
        {providers.map((provider) => (
          <ProviderRow
            key={provider.id}
            provider={provider}
            onToggle={(next) => void toggle(provider.id, next)}
          />
        ))}
      </div>
    </details>
  );
}

function ProviderRow({
  provider,
  onToggle,
}: {
  provider: Provider;
  onToggle: (next: boolean) => void;
}) {
  const { enabled, name, reads } = provider;
  return (
    <div className="flex items-center justify-between gap-s2">
      <span className="min-w-0">
        <span className="flex items-center gap-2 type-secondary">
          <span
            className="led"
            style={{
              color: enabled ? 'var(--status-ready)' : 'var(--status-idle)',
              background: enabled ? 'var(--status-ready)' : 'var(--status-idle)',
            }}
            aria-hidden
          />
          {name}
        </span>
        <span className="type-micro block text-app-muted">{reads.join(', ')}</span>
      </span>
      <button
        type="button"
        role="switch"
        aria-checked={enabled}
        aria-label={name}
        onClick={() => onToggle(!enabled)}
        className={`key h-8 shrink-0 px-3 type-micro ${enabled ? 'key-lit' : 'text-app-muted'}`}
      >
        {enabled ? 'On' : 'Off'}
      </button>
    </div>
  );
}
