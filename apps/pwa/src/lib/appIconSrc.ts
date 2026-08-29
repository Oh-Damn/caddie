import { loadSession } from '../companion/session';

export function appIconSrc(bundleId: string): string {
  const deviceId = loadSession()?.deviceId ?? '';
  return `/api/icon/${encodeURIComponent(bundleId)}?t=${encodeURIComponent(deviceId)}`;
}
