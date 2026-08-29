import type { Status } from '../../companion/useCompanion';

export const STATUS_TONE: Record<Status, string> = {
  idle: 'var(--status-idle)',
  connecting: 'var(--status-wait)',
  pairing: 'var(--status-wait)',
  ready: 'var(--status-ready)',
  error: 'var(--status-error)',
};

export const STATUS_LABEL: Record<Status, string> = {
  idle: 'Disconnected',
  connecting: 'Connecting',
  pairing: 'Pairing',
  ready: 'Connected',
  error: 'Connection error',
};

type Props = {
  status: Status;
};

export function ConnectionDot({ status }: Props) {
  const pulse = status === 'connecting' || status === 'pairing';
  return (
    <span
      className={`led ${pulse ? 'led-pulse' : ''}`}
      style={{ color: STATUS_TONE[status], background: STATUS_TONE[status] }}
      title={STATUS_LABEL[status]}
      aria-label={STATUS_LABEL[status]}
    />
  );
}
