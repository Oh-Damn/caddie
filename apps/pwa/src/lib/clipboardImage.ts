export function clipImageSrc(id: string, deviceId: string, full = false): string {
  const size = full ? '&size=full' : '';
  return `/api/clip/${encodeURIComponent(id)}?t=${encodeURIComponent(deviceId)}${size}`;
}
