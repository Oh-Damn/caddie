import type { ConnectedDevice } from '@companion/protocol';

type Props = {
  device: ConnectedDevice | null;
  connected: boolean;
};

function shortDate(iso: string): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return d.toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
}

function sinceLabel(iso: string): string {
  if (!iso) return '';
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return '';
  const mins = Math.floor((Date.now() - then) / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function modelLabel(device: ConnectedDevice): string {
  const version = device.platformVersion ? ` ${device.platformVersion}` : '';
  if (device.model) {
    return device.platform
      ? `${device.model} (${device.platform}${version})`
      : device.model;
  }
  return device.platform ? `${device.platform}${version}` : '';
}

export function DeviceCard({ device, connected }: Props) {
  if (!device) {
    return (
      <div className="faceplate mt-4 p-3 type-secondary">
        <p className="type-micro">Device</p>
        <p className="mt-1">No phone paired yet</p>
      </div>
    );
  }

  const model = modelLabel(device);
  const seen = sinceLabel(device.lastSeen);
  const paired = shortDate(device.pairedAt);

  return (
    <div className="faceplate mt-4 p-3 type-secondary">
      <p className="type-micro">{connected ? 'Connected' : 'Paired'}</p>
      <p className="type-secondary mt-1">{device.name || 'Phone'}</p>
      {model ? <p className="type-meta mt-0.5">{model}</p> : null}
      <dl className="type-meta mt-2 flex flex-col gap-0.5">
        {device.ip ? (
          <div className="flex justify-between gap-3">
            <dt>Address</dt>
            <dd className="font-mono text-lcd">{device.ip}</dd>
          </div>
        ) : null}
        {device.mac ? (
          <div className="flex justify-between gap-3">
            <dt>Wi-Fi MAC</dt>
            <dd className="font-mono text-lcd">{device.mac}</dd>
          </div>
        ) : null}
        {paired ? (
          <div className="flex justify-between gap-3">
            <dt>Paired</dt>
            <dd>{paired}</dd>
          </div>
        ) : null}
        {!connected && seen ? (
          <div className="flex justify-between gap-3">
            <dt>Last seen</dt>
            <dd>{seen}</dd>
          </div>
        ) : null}
      </dl>
    </div>
  );
}
