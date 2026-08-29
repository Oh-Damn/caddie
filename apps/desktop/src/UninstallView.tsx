import { PRODUCT_NAME } from '@companion/protocol';
import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import { Button } from './Button';

type Props = {
  bundled: boolean;
  onCancel: () => void;
};

export function UninstallView({ bundled, onCancel }: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const confirm = async () => {
    setBusy(true);
    setError(null);
    try {
      await invoke('uninstall');
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  return (
    <div className="flex min-h-screen flex-col bg-app px-6 py-7 text-app-text">
      <p className="type-micro">Remove</p>
      <h1 className="type-primary mt-2">Uninstall {PRODUCT_NAME}</h1>
      <p className="type-secondary mt-3 text-app-muted">
        This deletes pairing data and turns off Launch at Login
        {bundled ? `, then moves ${PRODUCT_NAME} to Trash` : ''}. The app will quit.
      </p>
      {!bundled ? (
        <p className="type-secondary mt-3 text-app-muted">
          This is a development build, so the project folder stays on disk.
        </p>
      ) : null}
      <p className="type-secondary mt-3 text-app-muted">
        Accessibility stays in System Settings until you remove {PRODUCT_NAME} there.
        Delete the phone home screen icon separately if you installed the PWA.
      </p>
      {error ? <p className="type-secondary mt-3 text-danger">{error}</p> : null}
      <div className="mt-auto flex justify-end gap-2 pt-8">
        <Button variant="ghost" onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
        <Button variant="danger" onClick={() => void confirm()} disabled={busy}>
          {busy ? 'Removing...' : 'Uninstall'}
        </Button>
      </div>
    </div>
  );
}
