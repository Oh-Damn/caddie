import { PRODUCT_NAME } from '@companion/protocol';
import { BrandMark } from '../components/ui/BrandMark';
import { Button } from '../components/ui/Button';
import { ConnectionDot } from '../components/ui/ConnectionDot';
import { Field } from '../components/ui/Field';
import { QrScan } from '../components/ui/QrScan';
import { useCompanion } from '../companion/useCompanion';
import { StackLayout } from '../layouts/StackLayout';

export function ConnectScreen() {
  const {
    host,
    setHost,
    secret,
    setSecret,
    connectManual,
    connectFromPairingUrl,
    status,
  } = useCompanion();
  const scanning = status === 'idle' || status === 'error';

  return (
    <div className="mx-auto flex min-h-0 max-w-md flex-1 flex-col overflow-hidden pt-8">
      <StackLayout>
        <div className="flex flex-col gap-5 pb-4">
          <p className="type-primary flex items-center gap-2">
            {PRODUCT_NAME}
            <ConnectionDot status={status} />
          </p>
          <BrandMark className="h-14 w-14" />
          <h1 className="type-primary text-[1.75rem]">Connect</h1>
          <p className="type-secondary text-app-muted">
            Scan the QR on the desktop app, or type the pairing code.
          </p>
          <QrScan active={scanning} onScan={connectFromPairingUrl} />
          <Field
            label="Pairing code"
            value={secret}
            onChange={setSecret}
            placeholder="from the desktop window"
          />
          <Button onClick={connectManual}>
            {status === 'connecting'
              ? 'Connecting...'
              : status === 'pairing'
                ? 'Pairing...'
                : 'Connect'}
          </Button>
          <details className="faceplate px-3 py-2">
            <summary className="type-micro min-h-touch cursor-pointer py-2">
              Different address
            </summary>
            <div className="flex flex-col gap-3 pb-2">
              <p className="type-meta text-app-muted">Connecting to {host}</p>
              <Field
                label="Desktop"
                value={host}
                onChange={setHost}
                placeholder="name or 192.168.1.5"
              />
            </div>
          </details>
        </div>
      </StackLayout>
    </div>
  );
}
